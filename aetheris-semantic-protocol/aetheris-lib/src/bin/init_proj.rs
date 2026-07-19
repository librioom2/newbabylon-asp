use std::fs::File;
use std::io::Write;

fn main() -> anyhow::Result<()> {
    let path = "projection.bin";
    let mut file = File::create(path)?;
    
    // Матрица 384 (E5) x 3072 (Phi-3.5)
    let size = 384 * 3072;
    let val = 0.1f32; // "Прозрачный" коэффициент для теста
    let bytes = val.to_le_bytes();

    println!("🛠️  Генерирую проекцию 384x3072 (веса: {})...", val);
    for _ in 0..size {
        file.write_all(&bytes)?;
    }
    
    println!("✅ Файл {} готов ({} байт).", path, size * 4);
    Ok(())
}

