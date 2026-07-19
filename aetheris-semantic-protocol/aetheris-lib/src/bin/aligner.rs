use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader, BufRead, Write};
use rayon::prelude::*;
use anyhow::Context;

fn main() -> anyhow::Result<()> {
    println!("🏗️  Aetheris Aligner: Стабильный мост E5 -> Phi-3.5...");

    let dim_in = 384;
    let dim_out = 3072;
    let lr = 0.0005;     // Еще более консервативный шаг для точности
    let epochs = 150;
    let clip = 0.05;     // Строгий клиппинг градиента

    // 1. Загрузка весов
    let e5_weights = load_f32_weights("layers_e5/token_embd.weight.f32.bin")?;
    let phi_weights = load_f32_weights("layers_phi/token_embd.weight.f32.bin")?;

    // 2. Загрузка и очистка словарей
    let e5_map = load_token_map("e5_tokens.txt")?;
    let phi_map = load_token_map("phi_tokens.txt")?;

    // 3. Поиск пар
    let mut shared_pairs = Vec::new();
    for (text, &e5_id) in &e5_map {
        if let Some(&phi_id) = phi_map.get(text) {
            if (e5_id + 1) * dim_in <= e5_weights.len() && (phi_id + 1) * dim_out <= phi_weights.len() {
                shared_pairs.push((e5_id, phi_id));
            }
        }
    }

    println!("📊 Валидных пар для обучения: {}", shared_pairs.len());

    // 4. Инициализация матрицы (маленький шум вместо нулей)
    let mut projection = vec![0.0f32; dim_out * dim_in];
    for val in projection.iter_mut() {
        *val = (rand_f32() - 0.5) * 0.001; 
    }

    // 5. Обучение
    println!("🚀 Запуск SGD (i9 Optimized)...");
    for epoch in 1..=epochs {
        projection.par_chunks_mut(dim_in)
            .enumerate()
            .for_each(|(out_idx, row)| {
                for &(e5_id, phi_id) in &shared_pairs {
                    // Извлекаем и нормализуем входной вектор E5
                    let mut input = e5_weights[e5_id * dim_in .. (e5_id + 1) * dim_in].to_vec();
                    normalize(&mut input);

                    // Извлекаем и нормализуем целевое значение Phi
                    let target = phi_weights[phi_id * dim_out + out_idx];
                    let target_clamped = target.clamp(-1.0, 1.0);

                    // Prediction (Dot Product)
                    let mut pred = 0.0;
                    for i in 0..dim_in {
                        pred += input[i] * row[i];
                    }

                    // Ошибка и обновление с клиппингом
                    let error = pred - target_clamped;
                    let diff = (error * lr).clamp(-clip, clip);

                    if !diff.is_nan() {
                        for i in 0..dim_in {
                            row[i] -= diff * input[i];
                        }
                    }
                }
            });

        if epoch % 25 == 0 {
            println!("Эпоха {}/{}... Статус: OK", epoch, epochs);
        }
    }

    // 6. Сохранение
    let mut out_file = File::create("layers_phi/projection.f32.bin")?;
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(projection.as_ptr() as *const u8, projection.len() * 4)
    };
    out_file.write_all(bytes)?;

    println!("✅ Мост готов: layers_phi/projection.f32.bin");
    Ok(())
}

// Вспомогательные функции
fn normalize(v: &mut [f32]) {
    let sum_sq: f32 = v.iter().map(|x| x * x).sum();
    let norm = sum_sq.sqrt();
    if norm > 1e-10 {
        for x in v.iter_mut() { *x /= norm; }
    }
}

fn rand_f32() -> f32 {
    // Простой генератор, чтобы не тянуть лишние зависимости
    static mut SEED: u32 = 42;
    unsafe {
        SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
        ((SEED >> 16) & 0x7FFF) as f32 / 32767.0
    }
}

fn load_f32_weights(path: &str) -> anyhow::Result<Vec<f32>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect())
}

fn load_token_map(path: &str) -> anyhow::Result<HashMap<String, usize>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut map = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        if let Some((text, id_str)) = line.rsplit_once(" : ") {
            if let Ok(id) = id_str.trim().parse::<usize>() {
                let clean = text.trim()
                    .replace(' ', "") // GGUF space
                    .replace(' ', "") // Normal space
                    .to_lowercase();
                if !clean.is_empty() { map.insert(clean, id); }
            }
        }
    }
    Ok(map)
}
