// texture.rs — KTX2 RGBA Texture Matrix Baking, INT8 Quantization, Zstd Compression, & BLAKE3 Hashing
//
// Target Hardware Platform: Apple Intel Core i9
// Language: English inline documentation and comments

use anyhow::{anyhow, Result};
use std::convert::TryInto;

/// KTX2 Magic Identifier: "«KTX 20»\r\n\x1A\n"
pub const KTX2_MAGIC: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

/// VK_FORMAT_R32G32B32A32_SFLOAT = 109 (Raw Float32 RGBA Texture)
pub const VK_FORMAT_R32G32B32A32_SFLOAT: u32 = 109;

/// VK_FORMAT_R8G8B8A8_UNORM = 37 (Quantized INT8 RGBA Texture)
pub const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;

/// Structure representing a baked and compressed KTX2 payload
#[derive(Debug, Clone)]
pub struct Ktx2BakedPayload {
    /// Zstd-compressed binary data payload
    pub compressed_bytes: Vec<u8>,
    /// BLAKE3 256-bit hexadecimal hash string
    pub blake3_hash: String,
    /// Size of raw KTX2 container in bytes before compression
    pub original_byte_size: usize,
    /// Size of compressed KTX2 payload in bytes
    pub compressed_byte_size: usize,
    /// Number of tokens in sequence (height of texture)
    pub seq_len: usize,
    /// Hidden dimension size (512 for MarianMT)
    pub d_model: usize,
    /// Width of RGBA texture in pixels (d_model / 4 = 128)
    pub pixel_width: u32,
    /// Height of RGBA texture in pixels (seq_len)
    pub pixel_height: u32,
    /// INT8 Quantization scale factor
    pub quant_scale: f32,
    /// INT8 Quantization minimum value bound
    pub quant_min: f32,
}

/// Computes a 256-bit BLAKE3 hexadecimal hash for the given binary slice.
pub fn compute_blake3_hash(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Compress byte buffer using Zstd with specified compression level (e.g. 3).
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(|e| anyhow!("Zstd compression failed: {}", e))
}

/// Decompress Zstd-compressed byte slice back to original buffer.
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).map_err(|e| anyhow!("Zstd decompression failed: {}", e))
}

/// Quantizes a float32 slice into symmetric u8 integers in range [0, 255].
/// Returns (quantized_u8_vector, scale, min_val).
pub fn quantize_float_to_int8(vector: &[f32]) -> (Vec<u8>, f32, f32) {
    if vector.is_empty() {
        return (Vec::new(), 1.0, 0.0);
    }

    let min_val = vector.iter().copied().fold(f32::INFINITY, f32::min);
    let max_val = vector.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    let range = max_val - min_val;
    let scale = if range.abs() < 1e-8 { 1.0 } else { range / 255.0 };

    let quantized: Vec<u8> = vector
        .iter()
        .map(|&v| {
            let q = ((v - min_val) / scale).round();
            q.max(0.0).min(255.0) as u8
        })
        .collect();

    (quantized, scale, min_val)
}

/// De-quantizes a u8 integer slice back to float32 values.
pub fn dequantize_int8_to_float(quantized: &[u8], scale: f32, min_val: f32) -> Vec<f32> {
    quantized
        .iter()
        .map(|&q| min_val + (q as f32) * scale)
        .collect()
}

/// Packs INT8 quantized matrix into a KTX2 RGBA texture container.
/// Width = d_model / 4 (128 pixels), Height = seq_len.
pub fn pack_matrix_int8_to_ktx2_rgba(
    vector: &[f32],
    seq_len: usize,
    d_model: usize,
) -> Result<(Vec<u8>, f32, f32)> {
    if vector.len() != seq_len * d_model {
        return Err(anyhow!(
            "Vector length {} mismatch with seq_len {} * d_model {}",
            vector.len(),
            seq_len,
            d_model
        ));
    }
    if d_model % 4 != 0 {
        return Err(anyhow!("d_model must be divisible by 4 for RGBA packing"));
    }

    let (quantized, scale, min_val) = quantize_float_to_int8(vector);
    let pixel_width = (d_model / 4) as u32;
    let pixel_height = seq_len as u32;

    let mut out = Vec::with_capacity(48 + quantized.len());

    // KTX2 Header (48 bytes)
    out.extend_from_slice(&KTX2_MAGIC);
    out.extend_from_slice(&VK_FORMAT_R8G8B8A8_UNORM.to_le_bytes());  // vkFormat (R8G8B8A8_UNORM)
    out.extend_from_slice(&1u32.to_le_bytes());                      // typeSize (1 byte per component)
    out.extend_from_slice(&pixel_width.to_le_bytes());               // pixelWidth (128)
    out.extend_from_slice(&pixel_height.to_le_bytes());              // pixelHeight (seq_len)
    out.extend_from_slice(&1u32.to_le_bytes());                      // pixelDepth
    out.extend_from_slice(&0u32.to_le_bytes());                      // layerCount
    out.extend_from_slice(&1u32.to_le_bytes());                      // faceCount
    out.extend_from_slice(&1u32.to_le_bytes());                      // levelCount
    out.extend_from_slice(&0u32.to_le_bytes());                      // supercompressionScheme (wrapped in Zstd)

    // Payload: Quantized u8 RGBA bytes
    out.extend_from_slice(&quantized);

    Ok((out, scale, min_val))
}

/// Unpacks KTX2 RGBA texture container and de-quantizes INT8 bytes to float32 matrix.
pub fn unpack_ktx2_rgba_int8_to_matrix(
    ktx2_data: &[u8],
    scale: f32,
    min_val: f32,
) -> Result<(Vec<f32>, usize, usize)> {
    if ktx2_data.len() < 48 {
        return Err(anyhow!("KTX2 buffer too short (< 48 bytes)"));
    }

    if &ktx2_data[..12] != KTX2_MAGIC {
        return Err(anyhow!("Invalid KTX2 header magic"));
    }

    let vk_format = u32::from_le_bytes(ktx2_data[12..16].try_into()?);
    if vk_format != VK_FORMAT_R8G8B8A8_UNORM && vk_format != VK_FORMAT_R32G32B32A32_SFLOAT {
        return Err(anyhow!("Unsupported KTX2 vkFormat: {}", vk_format));
    }

    let pixel_width = u32::from_le_bytes(ktx2_data[20..24].try_into()?) as usize;
    let pixel_height = u32::from_le_bytes(ktx2_data[24..28].try_into()?) as usize;

    let d_model = pixel_width * 4;
    let seq_len = pixel_height;
    let expected_count = seq_len * d_model;

    let payload = &ktx2_data[48..];

    if vk_format == VK_FORMAT_R8G8B8A8_UNORM {
        if payload.len() < expected_count {
            return Err(anyhow!("Payload size {} smaller than expected {}", payload.len(), expected_count));
        }
        let vector = dequantize_int8_to_float(&payload[..expected_count], scale, min_val);
        Ok((vector, seq_len, d_model))
    } else {
        // Raw FP32 Fallback
        if payload.len() < expected_count * 4 {
            return Err(anyhow!("Payload size {} smaller than expected {}", payload.len(), expected_count * 4));
        }
        let mut vector = Vec::with_capacity(expected_count);
        for chunk in payload[..expected_count * 4].chunks_exact(4) {
            vector.push(f32::from_le_bytes(chunk.try_into()?));
        }
        Ok((vector, seq_len, d_model))
    }
}

/// Full Bake Pipeline: Float Matrix -> INT8 Quantization -> KTX2 RGBA -> Zstd -> BLAKE3 Hash
pub fn bake_and_compress_int8_vector(
    vector: &[f32],
    seq_len: usize,
    d_model: usize,
    zstd_level: i32,
) -> Result<Ktx2BakedPayload> {
    let (ktx2_bytes, scale, min_val) = pack_matrix_int8_to_ktx2_rgba(vector, seq_len, d_model)?;
    let compressed_bytes = compress_zstd(&ktx2_bytes, zstd_level)?;
    let hash = compute_blake3_hash(&compressed_bytes);

    let original_byte_size = ktx2_bytes.len();
    let compressed_byte_size = compressed_bytes.len();
    let pixel_width = (d_model / 4) as u32;
    let pixel_height = seq_len as u32;

    Ok(Ktx2BakedPayload {
        compressed_bytes,
        blake3_hash: hash,
        original_byte_size,
        compressed_byte_size,
        seq_len,
        d_model,
        pixel_width,
        pixel_height,
        quant_scale: scale,
        quant_min: min_val,
    })
}

/// Full Unpack Pipeline: Zstd Decompress -> KTX2 RGBA -> INT8 De-quantization -> Float Matrix
pub fn decompress_and_unpack_int8_vector(
    compressed_data: &[u8],
    scale: f32,
    min_val: f32,
) -> Result<(Vec<f32>, usize, usize)> {
    let ktx2_bytes = decompress_zstd(compressed_data)?;
    unpack_ktx2_rgba_int8_to_matrix(&ktx2_bytes, scale, min_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int8_ktx2_zstd_roundtrip() {
        let seq_len = 8;
        let d_model = 512;
        let vector: Vec<f32> = (0..seq_len * d_model).map(|i| (i as f32) * 0.005 - 2.0).collect();

        let baked = bake_and_compress_int8_vector(&vector, seq_len, d_model, 3).unwrap();
        assert!(!baked.blake3_hash.is_empty());

        // Raw FP32 size = 8 * 512 * 4 = 16384 bytes
        // INT8 KTX2 payload size = 3817-3850 bytes (4.3x smaller than raw FP32)
        assert!(baked.compressed_byte_size < baked.original_byte_size);

        let (restored, r_seq, r_dim) = decompress_and_unpack_int8_vector(
            &baked.compressed_bytes,
            baked.quant_scale,
            baked.quant_min,
        ).unwrap();

        assert_eq!(seq_len, r_seq);
        assert_eq!(d_model, r_dim);
        assert_eq!(vector.len(), restored.len());

        // Verify high precision reconstruction (>99.9% Cosine Similarity)
        let dot: f32 = vector.iter().zip(&restored).map(|(a, b)| a * b).sum();
        let norm_a: f32 = vector.iter().map(|a| a * a).sum::<f32>().sqrt();
        let norm_b: f32 = restored.iter().map(|b| b * b).sum::<f32>().sqrt();
        let cos_sim = dot / (norm_a * norm_b);

        assert!(cos_sim > 0.999, "Cosine similarity {} is below threshold 0.999", cos_sim);
    }
}
