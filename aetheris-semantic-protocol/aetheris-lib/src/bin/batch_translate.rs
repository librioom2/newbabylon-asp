// batch_translate.rs
// Переводит словарь фраз из JSON-файла на все поддерживаемые языки.
//
// Использование:
//   cargo run --release -p aetheris-lib --bin batch_translate -- [phrases.json] [output.json]
//
// По умолчанию: phrases_100.json → translations_output.json

use anyhow::{Result, anyhow};
use aetheris_lib::ai::{SemanticEngine, DecoderEngine};
use std::path::Path;
use std::collections::HashMap;

/// Языки, для которых есть модели marian-en-XX
const TARGETS: &[(&str, &str)] = &[
    ("ru", "Russian"),
    ("de", "German"),
    ("fr", "French"),
    ("es", "Spanish"),
    ("zh", "Chinese"),
    ("ar", "Arabic"),
    ("uk", "Ukrainian"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
];

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input_path = args.get(1).map(|s| s.as_str()).unwrap_or("phrases_100.json");
    let output_path = args.get(2).map(|s| s.as_str()).unwrap_or("translations_output.json");

    // 1. Читаем фразы
    let raw = std::fs::read_to_string(input_path)
        .map_err(|e| anyhow!("Cannot read {}: {}", input_path, e))?;
    let phrases: Vec<String> = serde_json::from_str(&raw)?;
    println!("📖 Загружено {} фраз из {}", phrases.len(), input_path);

    // 2. Определяем доступные модели
    let mut available: Vec<(&str, &str)> = Vec::new();
    for &(code, name) in TARGETS {
        let model_dir = format!("models/marian-en-{}", code);
        if Path::new(&model_dir).exists() {
            available.push((code, name));
        } else {
            eprintln!("⚠️  Модель {} ({}) не найдена — пропускаем. Запустите `babylon init` для скачивания.", model_dir, name);
        }
    }

    if available.is_empty() {
        return Err(anyhow!("Нет доступных моделей. Запустите: cargo run --release -p babylon -- init"));
    }

    println!("🌍 Доступные языки: {}", available.iter().map(|(c, n)| format!("{} ({})", n, c)).collect::<Vec<_>>().join(", "));
    println!("═══════════════════════════════════════════════════════");

    // 3. Структура результата: { "phrase": { "lang_code": "translation", ... }, ... }
    let mut results: Vec<HashMap<String, String>> = Vec::new();
    for phrase in &phrases {
        let mut entry = HashMap::new();
        entry.insert("en".to_string(), phrase.clone());
        results.push(entry);
    }

    let total_start = std::time::Instant::now();

    // 4. Для каждого языка: загрузить модель один раз, перевести все фразы
    for &(code, name) in &available {
        let model_dir = format!("models/marian-en-{}", code);
        let model_path = Path::new(&model_dir);

        println!("\n🔄 [{} — {}] Загрузка модели...", code.to_uppercase(), name);
        let load_start = std::time::Instant::now();
        let mut encoder = SemanticEngine::load(model_path)?;
        let mut decoder = DecoderEngine::new(model_path)?;
        println!("   ⏱️  Модель загружена за {:?}", load_start.elapsed());

        let lang_start = std::time::Instant::now();
        let mut success_count = 0usize;
        let mut fail_count = 0usize;

        for (i, phrase) in phrases.iter().enumerate() {
            match translate_phrase(&mut encoder, &mut decoder, phrase) {
                Ok(translation) => {
                    // Прогресс каждые 10 фраз
                    if (i + 1) % 10 == 0 || i == 0 {
                        println!("   [{:3}/{}] \"{}\" → \"{}\"", i + 1, phrases.len(), 
                            truncate(phrase, 40), truncate(&translation, 50));
                    }
                    results[i].insert(code.to_string(), translation);
                    success_count += 1;
                }
                Err(e) => {
                    eprintln!("   ❌ [{:3}] Ошибка: {} — {}", i + 1, truncate(phrase, 30), e);
                    results[i].insert(code.to_string(), format!("[ERROR: {}]", e));
                    fail_count += 1;
                }
            }
        }

        println!("   ✅ {} переведено, ❌ {} ошибок, ⏱️  {:?}", success_count, fail_count, lang_start.elapsed());
    }

    let total_elapsed = total_start.elapsed();

    // 5. Сохраняем результат
    let output_json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, &output_json)?;

    println!("\n═══════════════════════════════════════════════════════");
    println!("🏁 Готово! {} фраз × {} языков = {} переводов", 
        phrases.len(), available.len(), phrases.len() * available.len());
    println!("⏱️  Общее время: {:?}", total_elapsed);
    println!("💾 Результат сохранён в {}", output_path);

    Ok(())
}

fn translate_phrase(
    encoder: &mut SemanticEngine,
    decoder: &mut DecoderEngine,
    text: &str,
) -> Result<String> {
    let (vector, seq_len, d_model) = encoder.encode(text)?;
    let translation = decoder.decode(&vector, seq_len, d_model)?;
    Ok(translation)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}
