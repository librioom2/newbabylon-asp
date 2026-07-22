use anyhow::{Result, anyhow};
use std::path::Path;
use tokenizers::Tokenizer;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::marian::{MTModel, Config};

pub struct SemanticEngine {
    pub model: MTModel,
    pub tokenizer: Tokenizer,
    pub device: Device,
}

pub struct DecoderEngine {
    pub model: MTModel,
    pub tokenizer: Tokenizer,
    pub config: Config,
    pub device: Device,
}

impl SemanticEngine {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let device = Device::Cpu;
        
        // Загружаем конфигурацию
        let config_path = model_dir.join("config.json");
        let config_file = std::fs::File::open(&config_path)
            .map_err(|e| anyhow!("Failed to open config {:?}: {}", config_path, e))?;
        let config: Config = serde_json::from_reader(config_file)?;

        // Загружаем токенизатор
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer {:?}: {}", tokenizer_path, e))?;

        // Загружаем веса модели через VarBuilder
        let safetensors_path = model_dir.join("model.safetensors");
        let bin_path = model_dir.join("pytorch_model.bin");
        let vb = if safetensors_path.exists() {
            unsafe { VarBuilder::from_mmaped_safetensors(&[safetensors_path], candle_core::DType::F32, &device)? }
        } else if bin_path.exists() {
            VarBuilder::from_pth(&bin_path, candle_core::DType::F32, &device)?
        } else {
            return Err(anyhow!("Neither model.safetensors nor pytorch_model.bin found in {:?}", model_dir));
        };

        // Инициализируем модель
        let model = MTModel::new(&config, vb)?;

        println!("🧠 MarianMT Encoder готов. Размер словаря: {}", config.vocab_size);
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn encode(&mut self, text: &str) -> Result<(Vec<f32>, usize, usize)> {
        let tokens = self.tokenizer.encode(text, true)
            .map_err(|e| anyhow!("Tokenization error: {}", e))?;
        let token_ids = tokens.get_ids();
        if token_ids.is_empty() {
            return Err(anyhow!("Empty input text"));
        }
        
        let input_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let encoder_outputs = self.model.encoder().forward(&input_ids, 0)?;
        
        let shape = encoder_outputs.shape();
        let seq_len = shape.dims()[1];
        let d_model = shape.dims()[2];
        
        let flat_data = encoder_outputs.flatten_all()?.to_vec1::<f32>()?;
        
        Ok((flat_data, seq_len, d_model))
    }

    pub fn pack_semantic_vector(&self, vector: &[f32], _session_id: u64, _ghost_hash: u64) -> Vec<u8> {
        vector.iter().flat_map(|&f| f.to_le_bytes().to_vec()).collect()
    }
}

impl DecoderEngine {
    pub fn new(model_dir: &Path) -> Result<Self> {
        let device = Device::Cpu;
        
        // Загружаем конфигурацию
        let config_path = model_dir.join("config.json");
        let config_file = std::fs::File::open(&config_path)
            .map_err(|e| anyhow!("Failed to open config {:?}: {}", config_path, e))?;
        let config: Config = serde_json::from_reader(config_file)?;

        // Загружаем токенизатор
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer {:?}: {}", tokenizer_path, e))?;

        // Загружаем веса модели через VarBuilder
        let safetensors_path = model_dir.join("model.safetensors");
        let bin_path = model_dir.join("pytorch_model.bin");
        let vb = if safetensors_path.exists() {
            unsafe { VarBuilder::from_mmaped_safetensors(&[safetensors_path], candle_core::DType::F32, &device)? }
        } else if bin_path.exists() {
            VarBuilder::from_pth(&bin_path, candle_core::DType::F32, &device)?
        } else {
            return Err(anyhow!("Neither model.safetensors nor pytorch_model.bin found in {:?}", model_dir));
        };

        // Инициализируем модель
        let model = MTModel::new(&config, vb)?;

        println!("🗣️ MarianMT Decoder готов. Язык перевода определен конфигурацией.");
        Ok(Self {
            model,
            tokenizer,
            config,
            device,
        })
    }

    pub fn decode(&mut self, flat_encoder_xs: &[f32], seq_len: usize, d_model: usize) -> Result<String> {
        let encoder_xs = Tensor::from_vec(flat_encoder_xs.to_vec(), &[1, seq_len, d_model], &self.device)?;
        
        self.model.reset_kv_cache();
        
        let mut tokens = vec![self.config.decoder_start_token_id];
        let mut past_kv_len = 0;
        
        for _ in 0..128 { // лимит генерации
            let ys = if past_kv_len == 0 {
                Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?
            } else {
                Tensor::new(&[tokens[tokens.len() - 1]], &self.device)?.unsqueeze(0)?
            };
            
            let logits = self.model.decode(&ys, &encoder_xs, past_kv_len)?;
            let current_seq_len = logits.narrow(1, logits.dim(1)? - 1, 1)?;
            let last_logits = current_seq_len.squeeze(1)?.squeeze(0)?;
            
            let next_token = last_logits.argmax(0)?
                .to_scalar::<u32>()?;
            
            if next_token == self.config.eos_token_id || next_token == self.config.forced_eos_token_id {
                break;
            }
            
            tokens.push(next_token);
            past_kv_len += ys.dim(1)?;
        }
        
        // Декодируем токены (пропускаем первый start_token_id)
        let text = self.tokenizer.decode(&tokens[1..], true)
            .map_err(|e| anyhow!("Tokenizer decoding error: {}", e))?;
        
        Ok(text)
    }
}

// Модуль DESS (Dynamic Embedding Space Shuffling)
pub struct DessModule {
    seed: u64,
}

impl DessModule {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn shuffle(&self, vector: &mut [f32]) {
        use rand::{seq::SliceRandom, SeedableRng};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(self.seed);
        vector.shuffle(&mut rng);
    }

    pub fn unshuffle(&self, vector: &mut [f32]) {
        use rand::{seq::SliceRandom, SeedableRng};
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(self.seed);
        let mut indices: Vec<usize> = (0..vector.len()).collect();
        indices.shuffle(&mut rng);
        
        let mut original = vec![0.0; vector.len()];
        for (i, &shuffled_idx) in indices.iter().enumerate() {
            if shuffled_idx < original.len() {
                original[shuffled_idx] = vector[i];
            }
        }
        vector.copy_from_slice(&original);
    }
}
