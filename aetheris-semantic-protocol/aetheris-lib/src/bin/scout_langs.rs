use std::fs::File;
use std::io::BufReader;
use serde_json::Value;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let file = File::open("models/tokenizer.json")?;
    let reader = BufReader::new(file);
    let json: Value = serde_json::from_reader(reader)?;

    let vocab = json["model"]["vocab"]
        .as_object()
        .expect("Не найден vocab");

    let mut ranges: BTreeMap<&str, (u32, u32)> = BTreeMap::new();

    for (token, id_val) in vocab {
        let id = id_val.as_u64().unwrap() as u32;
        
        // Упрощенная проверка по Unicode блокам
        let lang = if token.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)) {
            "RU"
        } else if token.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c)) {
            "ZH"
        } else if token.chars().any(|c| c.is_ascii_alphabetic()) {
            "EN"
        } else if token.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)) {
            "AR"
        } else {
            "OTHER"
        };

        let entry = ranges.entry(lang).or_insert((id, id));
        if id < entry.0 { entry.0 = id; }
        if id > entry.1 { entry.1 = id; }
    }

    println!("🗺️  Языковая карта Phi-3.5:");
    for (lang, (min, max)) in &ranges {
        println!("{:<10} | ID: {:<6} .. {:<6} | Tokens: {}", lang, min, max, max - min);
    }

    Ok(())
}

