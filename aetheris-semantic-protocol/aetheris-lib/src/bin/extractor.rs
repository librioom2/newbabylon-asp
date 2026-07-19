use std::io::{self, BufRead, Read, Write, Seek, SeekFrom};
use std::fs::{self, File};
use std::path::Path;
use half::f16;
use serde::Serialize;
use anyhow::{Context, Result};

#[derive(Serialize)]
struct TensorMeta {
    name: String,
    file: String,
    size: usize,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Использование: scout <model.gguf> | extractor <model.gguf> <output_dir>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_dir = &args[2];
    fs::create_dir_all(output_dir).context("Не удалось создать директорию слоев")?;

    let mut model_file = File::open(input_path).context("Не удалось открыть GGUF")?;
    let mut extracted_tensors = Vec::new();

    println!("--- [FLUID EXTRACTOR: DEQUANTIZATION MODE] ---");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 4 || parts[0] == "name" || parts[0].starts_with('-') {
            continue; 
        }

        let name = parts[0];
        let kind_id: u8 = parts[1].parse().unwrap_or(255);
        let offset: u64 = parts[2].parse().unwrap_or(0);
        let size: usize = parts[3].parse().unwrap_or(0);

        if size == 0 { continue; }

        let out_file_name = format!("{}.f32.bin", name.replace('.', "_"));
        let out_path = Path::new(output_dir).join(&out_file_name);

        model_file.seek(SeekFrom::Start(offset))?;
        let mut buffer = vec![0u8; size];
        model_file.read_exact(&mut buffer)?;

        // Основная логика выбора деквантователя
        let f32_data = match kind_id {
            0 => { // F32
                buffer.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect()
            },
            1 => dequantize_f16(&buffer),     // F16 -> F32
            8 => dequantize_q8_0(&buffer),    // Q8_0 -> F32
            12 => dequantize_q4_k(&buffer),   // Q4_K -> F32
            13 => dequantize_q5_k(&buffer),   // Q5_K -> F32
            14 => dequantize_q6_k(&buffer),   // Q6_K -> F32
            _ => {
                println!("⚠️ Пропуск {}: тип {} не поддерживается", name, kind_id);
                continue;
            }
        };

        let mut out_file = File::create(&out_path)?;
        // Записываем данные как байты f32
        let bytes: &[u8] = bytemuck::cast_slice(&f32_data);
        out_file.write_all(bytes)?;

        extracted_tensors.push(TensorMeta {
            name: name.to_string(),
            file: out_file_name,
            size: bytes.len(),
        });

        println!("💎 Sliced: {:<35} | Type: {:<2} -> F32 | Size: {} bytes", name, kind_id, bytes.len());
    }

    let manifest_json = serde_json::to_string_pretty(&extracted_tensors)?;
    fs::write(Path::new(output_dir).join("manifest.json"), manifest_json)?;

    println!("--- [COMPLETED] --- Слои готовы в: {}", output_dir);
    Ok(())
}

// --- Функции деквантования ---

fn dequantize_f16(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(2)
        .map(|c| f16::from_bits(u16::from_le_bytes([c[0], c[1]])).to_f32())
        .collect()
}

fn dequantize_q8_0(data: &[u8]) -> Vec<f32> {
    let mut output = Vec::with_capacity(data.len() / 34 * 32);
    for block in data.chunks_exact(34) {
        let d = f16::from_bits(u16::from_le_bytes([block[0], block[1]])).to_f32();
        for i in 2..34 {
            output.push((block[i] as i8) as f32 * d);
        }
    }
    output
}

fn dequantize_q4_k(data: &[u8]) -> Vec<f32> {
    // В Q4_K блок 176 байт содержит 256 значений
    let mut output = Vec::with_capacity((data.len() / 176) * 256);
    for block in data.chunks_exact(176) {
        let d = f16::from_bits(u16::from_le_bytes([block[0], block[1]])).to_f32();
        let dmin = f16::from_bits(u16::from_le_bytes([block[2], block[3]])).to_f32();
        let scales = &block[4..16]; 
        let qs = &block[48..];

        for i in 0..256 {
            let group = i / 32;
            let sc = if group < 4 { scales[group * 2] & 0x3f } else { scales[(group-4) * 2 + 1] & 0x3f };
            let m  = if group < 4 { scales[group * 2 + 1] & 0x3f } else { scales[(group-4) * 2] >> 6 | ((scales[(group-4) * 2 + 1] >> 6) << 2) };
            let q = if i % 2 == 0 { qs[i/2] & 0x0F } else { qs[i/2] >> 4 };
            output.push(d * (sc as f32) * (q as f32) - dmin * (m as f32));
        }
    }
    output
}

fn dequantize_q5_k(data: &[u8]) -> Vec<f32> {
    let mut output = Vec::with_capacity((data.len() / 208) * 256);
    for block in data.chunks_exact(208) {
        let d = f16::from_bits(u16::from_le_bytes([block[0], block[1]])).to_f32();
        let dmin = f16::from_bits(u16::from_le_bytes([block[2], block[3]])).to_f32();
        let qh = &block[16..48];
        let qs = &block[48..];

        for i in 0..256 {
            let mut q = if i % 2 == 0 { qs[i/2] & 0x0F } else { qs[i/2] >> 4 };
            if (qh[i/8] & (1 << (i%8))) != 0 { q |= 0x10; }
            output.push(d * (q as f32) - dmin);
        }
    }
    output
}

fn dequantize_q6_k(data: &[u8]) -> Vec<f32> {
    let mut output = Vec::with_capacity((data.len() / 210) * 256);
    for block in data.chunks_exact(210) {
        let d = f16::from_bits(u16::from_le_bytes([block[0], block[1]])).to_f32();
        let ql = &block[2..130];
        let qh = &block[130..194];
        let scales = &block[194..210];

        for i in 0..256 {
            let sc = scales[i / 32] as f32;
            let mut q = (ql[i/2] >> (if i % 2 == 0 { 0 } else { 4 })) & 0x0F;
            let h_bit = (qh[i/4] >> (2 * (i%4))) & 0x03;
            q |= h_bit << 4;
            output.push(d * sc * (q as i8 - 32) as f32);
        }
    }
    output
}
