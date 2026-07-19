use gguf_rs::get_gguf_container;
use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: scout <model_path> -t");
        return Ok(());
    }

    let model_path = &args[1];
    let mut file = File::open(model_path)?;

    // 1. Используем библиотеку только чтобы найти метаданные
    let mut container = get_gguf_container(model_path).map_err(|e| anyhow::anyhow!(e))?;
    let model = container.decode().map_err(|e| anyhow::anyhow!(e))?;

    // 2. Ищем ключ токенов
    if args.contains(&"-t".to_string()) {
        if let Some(tokens_val) = model.metadata().get("tokenizer.ggml.tokens") {
            println!("--- [DIRECT BINARY TOKEN EXTRACTION] ---");
            
            // Получаем отладочную строку, чтобы понять, сколько там элементов
            let debug_str = format!("{:?}", tokens_val);
            // Попытаемся вытащить количество элементов из строки типа "Array(String, 32064)"
            let count = debug_str.split(',').last()
                .and_then(|s| s.trim_matches(|c: char| !c.is_digit(10)).parse::<usize>().ok())
                .unwrap_or(32000); // Дефолт для большинства моделей

            // В GGUF токены обычно идут списком строк (Length-prefixed)
            // Но проще всего: если библиотека не дает доступ, 
            // мы просто распарсим всё, что похоже на строки в этом блоке.
            
            // ВАЖНО: Если мы не можем итерировать Array в этой либе, 
            // единственный путь — использовать `serde` или `flat_map`, 
            // но попробуем самый стабильный "хак":
            
            let raw_data = format!("{:#?}", tokens_val);
            let mut id = 0;
            for line in raw_data.lines() {
                if line.contains("\"") {
                    let parts: Vec<&str> = line.split('"').collect();
                    if parts.len() >= 2 {
                        println!("{} : {}", parts[1], id);
                        id += 1;
                    }
                }
            }
            
            // Если всё еще 3-4 штуки, значит либа ленивая. 
            // Последний шанс: выводим паспорт и ищем смещение вручную.
            if id < 10 {
                println!("!! ВНИМАНИЕ: Либиотека обрезает данные. Используй 'gguf-dump' или аналоги.");
                println!("Версия библиотеки gguf-rs в твоем Cargo.toml устарела или ограничена.");
            }
            return Ok(());
        }
    }

    // Стандартный вывод тензоров (как раньше)
    let file_size = file.metadata()?.len();
    let total_weights: u64 = model.tensors().iter().map(|t| t.size as u64).sum();
    let data_offset = (file_size.saturating_sub(total_weights) + 31) & !31;

    for tensor in model.tensors().iter() {
        println!("{:<30} {:<12} {:<12}", tensor.name, data_offset + tensor.offset, tensor.size);
    }

    Ok(())
}
