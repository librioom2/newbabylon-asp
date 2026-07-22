// texture.rs — KTX2 RGBA Texture Matrix Packing & Zstd Compression
use anyhow::{anyhow, Result};
use std::convert::TryInto;

/// KTX2 Magic Identifier: "«KTX 20»\r\n\x1A\n"
pub const KTX2_MAGIC: [u8; 12] = [
    0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
];

/// VK_FORMAT_R32G32B32A32_SFLOAT = 109
pub const VK_FORMAT_R32G32B32A32_SFLOAT: u32 = 109;

#[derive(Debug, Clone)]
pub struct Ktx2BakedPayload {
    pub compressed_bytes: Vec<u8>,
    pub blake3_hash: String,
    pub original_byte_size: usize,
    pub compressed_byte_size: usize,
    pub seq_len: usize,
    pub d_model: usize,
    pub pixel_width: u32,
    pub pixel_height: u32,
}

/// Computes BLAKE3 hex hash of binary buffer
pub fn compute_blake3_hash(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Compress byte buffer using Zstd
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(|e| anyhow!("Zstd compression error: {}", e))
}

/// Decompress Zstd buffer
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data).map_err(|e| anyhow!("Zstd decompression error: {}", e))
}

/// Packs float vector matrix [L, D] into KTX2 RGBA texture format.
/// Width = D / 4 (128 pixels for D=512), Height = L (sequence length).
pub fn pack_matrix_to_ktx2_rgba(
    vector: &[f32],
    seq_len: usize,
    d_model: usize,
) -> Result<Vec<u8>> {
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

    let pixel_width = (d_model / 4) as u32;
    let pixel_height = seq_len as u32;

    let mut out = Vec::with_capacity(48 + vector.len() * 4);

    // KTX2 Header (48 bytes)
    out.extend_from_slice(&KTX2_MAGIC);
    out.extend_from_slice(&VK_FORMAT_R32G32B32A32_SFLOAT.to_le_bytes()); // vkFormat
    out.extend_from_slice(&4u32.to_le_bytes());                          // typeSize (4 bytes float)
    out.extend_from_slice(&pixel_width.to_le_bytes());                   // pixelWidth
    out.extend_from_slice(&pixel_height.to_le_bytes());                  // pixelHeight
    out.extend_from_slice(&1u32.to_le_bytes());                          // pixelDepth
    out.extend_from_slice(&0u32.to_le_bytes());                          // layerCount
    out.extend_from_slice(&1u32.to_le_bytes());                          // faceCount
    out.extend_from_slice(&1u32.to_le_bytes());                          // levelCount
    out.extend_from_slice(&0u32.to_le_bytes());                          // supercompressionScheme (0=none, wrapped in zstd)

    // Payload: Raw float data (RGBA channels)

    for &f in vector {
        out.extend_from_slice(&f.to_le_bytes());
    }

    Ok(out)
}

/// Unpacks KTX2 RGBA texture buffer into float matrix [L, D].
pub fn unpack_ktx2_rgba_to_matrix(ktx2_data: &[u8]) -> Result<(Vec<f32>, usize, usize)> {
    if ktx2_data.len() < 48 {
        return Err(anyhow!("KTX2 buffer too short (< 48 bytes)"));
    }

    if &ktx2_data[..12] != KTX2_MAGIC {
        return Err(anyhow!("Invalid KTX2 header magic"));
    }

    let vk_format = u32::from_le_bytes(ktx2_data[12..16].try_into()?);
    if vk_format != VK_FORMAT_R32G32B32A32_SFLOAT {
        return Err(anyhow!("Unsupported KTX2 format: {}", vk_format));
    }

    let pixel_width = u32::from_le_bytes(ktx2_data[20..24].try_into()?) as usize;
    let pixel_height = u32::from_le_bytes(ktx2_data[24..28].try_into()?) as usize;

    let d_model = pixel_width * 4;
    let seq_len = pixel_height;
    let expected_floats = seq_len * d_model;

    let payload = &ktx2_data[48..];
    if payload.len() < expected_floats * 4 {
        return Err(anyhow!(
            "KTX2 payload length {} is smaller than expected {}",
            payload.len(),
            expected_floats * 4
        ));
    }

    let mut vector = Vec::with_capacity(expected_floats);
    for chunk in payload[..expected_floats * 4].chunks_exact(4) {
        vector.push(f32::from_le_bytes(chunk.try_into()?));
    }

    Ok((vector, seq_len, d_model))
}

/// Full Bake Pipeline: Float Matrix -> KTX2 RGBA -> Zstd -> BLAKE3 Hash
pub fn bake_and_compress_vector(
    vector: &[f32],
    seq_len: usize,
    d_model: usize,
    zstd_level: i32,
) -> Result<Ktx2BakedPayload> {
    let ktx2_bytes = pack_matrix_to_ktx2_rgba(vector, seq_len, d_model)?;
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
    })
}

/// Full Unpack Pipeline: Zstd Decompress -> KTX2 RGBA -> Float Matrix
pub fn decompress_and_unpack_vector(compressed_data: &[u8]) -> Result<(Vec<f32>, usize, usize)> {
    let ktx2_bytes = decompress_zstd(compressed_data)?;
    unpack_ktx2_rgba_to_matrix(&ktx2_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ktx2_zstd_roundtrip() {
        let seq_len = 8;
        let d_model = 512;
        let vector: Vec<f32> = (0..seq_len * d_model).map(|i| (i as f32) * 0.01).collect();

        let baked = bake_and_compress_vector(&vector, seq_len, d_model, 3).unwrap();
        assert!(!baked.blake3_hash.is_empty());
        assert!(baked.compressed_byte_size < baked.original_byte_size);

        let (restored, r_seq, r_dim) = decompress_and_unpack_vector(&baked.compressed_bytes).unwrap();
        assert_eq!(seq_len, r_seq);
        assert_eq!(d_model, r_dim);
        assert_eq!(vector, restored);
    }
}
