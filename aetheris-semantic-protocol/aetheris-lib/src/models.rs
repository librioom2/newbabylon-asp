use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Downloader;

impl Downloader {
    /// Downloads a MarianMT model by repo_id into the specified directory using curl
    pub fn download_model(repo_id: &str, dest_dir: &str) -> anyhow::Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
        let root = Path::new("models").join(dest_dir);
        if !root.exists() {
            fs::create_dir_all(&root)?;
        }
        // Основные файлы весов и конфигурации скачиваем из оригинального репозитория
        let files = ["config.json", "vocab.json", "pytorch_model.bin"];
        let mut paths = Vec::new();
        println!("📡 Загрузка файлов для {} в {:?}...", repo_id, root);
        for file in &files {
            let dest_path = root.join(file);
            if !dest_path.exists() {
                let url = format!("https://huggingface.co/{}/resolve/main/{}", repo_id, file);
                println!("📥 Скачивание {}...", url);
                let status = Command::new("curl")
                    .args(["-L", "-o", dest_path.to_str().unwrap(), &url])
                    .status()?;
                if !status.success() {
                    return Err(anyhow::anyhow!("Failed to download {} via curl", file));
                }
            } else {
                println!("✅ {} уже существует.", file);
            }
            paths.push(dest_path);
        }
        // tokenizer.json скачиваем из репозитория onnx-community (так как Helsinki-NLP содержит только .spm файлы, которые Candle не поддерживает)
        let tok_path = root.join("tokenizer.json");
        if !tok_path.exists() {
            let model_name = repo_id.split('/').last().unwrap();
            let tok_url = format!("https://huggingface.co/onnx-community/{}/resolve/main/tokenizer.json", model_name);
            println!("📥 Скачивание tokenizer.json из {}...", tok_url);
            let status = Command::new("curl")
                .args(["-L", "-o", tok_path.to_str().unwrap(), &tok_url])
                .status()?;
            if !status.success() {
                // Fallback: generate tokenizer.json from original repo using convert_slow_tokenizer
                println!("⚠️ tokenizer.json не найден в onnx-community, пытаемся конвертировать из оригинального репозитория.");
                // download vocab and merges if needed
                let vocab_path = root.join("vocab.json");
                let merges_path = root.join("merges.txt");
                // download merges.txt if not present (some models use merges)
                if !merges_path.exists() {
                    let merges_url = format!("https://huggingface.co/{}/resolve/main/merges.txt", repo_id);
                    let _ = Command::new("curl")
                        .args(["-L", "-o", merges_path.to_str().unwrap(), &merges_url])
                        .status();
                }
                // run Python conversion script
                let conversion_status = Command::new("python3")
                    .args([
                        "-c",
                        &format!(
                            "from transformers import MarianTokenizer;import json;model='{}';tokenizer=MarianTokenizer.from_pretrained(model);json.dump(tokenizer.get_vocab(), open('{}','w'))",
                            repo_id,
                            tok_path.to_str().unwrap()
                        ),
                    ])
                    .status()?;
                if !conversion_status.success() {
                    return Err(anyhow::anyhow!("Failed to generate tokenizer.json via Python conversion"));
                }
            }
        } else {
            println!("✅ tokenizer.json уже существует.");
        }
        Ok((paths[0].clone(), tok_path, paths[1].clone(), paths[2].clone()))
    }

    /// Removes all previously downloaded MarianMT model directories (prefixed with "marian-")
    pub fn remove_old_models() -> anyhow::Result<()> {
        let models_root = Path::new("models");
        if models_root.exists() {
            for entry in fs::read_dir(models_root)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("marian-") {
                            println!("🗑️ Удаление старой модели: {:?}", path);
                            fs::remove_dir_all(&path)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Helper to download a sed2sed model (placeholder implementation – reuses Marian download logic)
    pub fn download_sed2sed_model(repo_id: &str, dest_dir: &str) -> anyhow::Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
        // For now we reuse the same download logic; in a real scenario the file list would differ (e.g., model.onnx)
        println!("🔄 Downloading sed2sed model {} into {}", repo_id, dest_dir);
        Self::download_model(repo_id, dest_dir)
    }

    /// Downloads all sed2sed models after cleaning old ones
    pub fn fetch_sed2sed_all() -> anyhow::Result<()> {
        // Clean old Marian models first
        Self::remove_old_models()?;
        // Example sed2sed repo IDs – replace with actual ones when available
        Self::download_sed2sed_model("sed2sed/en-ru", "sed2sed-en-ru")?;
        Self::download_sed2sed_model("sed2sed/ru-en", "sed2sed-ru-en")?;
        Self::download_sed2sed_model("sed2sed/en-de", "sed2sed-en-de")?;
        Self::download_sed2sed_model("sed2sed/en-fr", "sed2sed-en-fr")?;
        Self::download_sed2sed_model("sed2sed/en-es", "sed2sed-en-es")?;
        Self::download_sed2sed_model("sed2sed/en-zh", "sed2sed-en-zh")?;
        Self::download_sed2sed_model("sed2sed/en-ar", "sed2sed-en-ar")?;
        Self::download_sed2sed_model("sed2sed/en-uk", "sed2sed-en-uk")?;
        Self::download_sed2sed_model("sed2sed/en-ja", "sed2sed-en-ja")?;
        Self::download_sed2sed_model("sed2sed/en-ko", "sed2sed-en-ko")?;
        Ok(())
    }

    /// Helper method to download all default team encoders and decoders
    pub fn fetch_all() -> anyhow::Result<()> {
        // Encoded models (Source Language -> English Pivot Space)
        Self::download_model("Helsinki-NLP/opus-mt-ru-en", "encoders/marian-ru-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-de-en", "encoders/marian-de-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-fr-en", "encoders/marian-fr-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-es-en", "encoders/marian-es-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-zh-en", "encoders/marian-zh-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-ar-en", "encoders/marian-ar-en")?;

        // Decoded models (English Pivot Space -> Target Language)
        Self::download_model("Helsinki-NLP/opus-mt-en-ru", "decoders/marian-en-ru")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-de", "decoders/marian-en-de")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-fr", "decoders/marian-en-fr")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-es", "decoders/marian-en-es")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-zh", "decoders/marian-en-zh")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-ar", "decoders/marian-en-ar")?;

        // Legacy compatibility aliases
        Self::download_model("Helsinki-NLP/opus-mt-en-ru", "marian-en-ru")?;
        Self::download_model("Helsinki-NLP/opus-mt-ru-en", "marian-ru-en")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-de", "marian-en-de")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-fr", "marian-en-fr")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-es", "marian-en-es")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-zh", "marian-en-zh")?;
        Self::download_model("Helsinki-NLP/opus-mt-en-ar", "marian-en-ar")?;
        Ok(())
    }
}

// Enum of supported target languages for translate-multi command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLang {
    Ru,
    De,
    Fr,
    Es,
    Zh,
    Ar,
    Uk,
    Ja,
    Ko,
}

impl SupportedLang {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupportedLang::Ru => "ru",
            SupportedLang::De => "de",
            SupportedLang::Fr => "fr",
            SupportedLang::Es => "es",
            SupportedLang::Zh => "zh",
            SupportedLang::Ar => "ar",
            SupportedLang::Uk => "uk",
            SupportedLang::Ja => "ja",
            SupportedLang::Ko => "ko",
        }
    }
}
