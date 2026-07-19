use std::fs::File;
use std::io::{Read, Seek, SeekFrom, BufWriter};

fn extract(input: &str, output: &str, offset: u64, size: u64) -> anyhow::Result<()> {
    let mut infile = File::open(input)?;
    let outfile = File::create(output)?;
    let mut writer = BufWriter::new(outfile);

    println!("📥 Извлечение из {}: смещение {}, размер {} байт...", input, offset, size);
    
    // Прыгаем на нужную позицию
    infile.seek(SeekFrom::Start(offset))?;
    
    // Ограничиваем чтение нужным размером и копируем в буферизированный поток
    let mut handle = infile.take(size);
    std::io::copy(&mut handle, &mut writer)?;
    
    println!("✅ Готово: {}", output);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Создаем директории, если их нет
    std::fs::create_dir_all("layers_phi")?;
    std::fs::create_dir_all("layers_e5")?;

    // Данные из твоего scout для Phi-3.5
    extract(
        "models/phi3_5.gguf", 
        "layers_phi/token_embd.weight.f32.bin", 
        7132224, 
        192028416
    )?;

    // Данные из твоего scout для E5
    extract(
        "models/multilingual-e5-small-F16.gguf", 
        "layers_e5/token_embd.weight.f32.bin", 
        738720, 
        55406592
    )?;

    Ok(())
}
