use clap::{Parser, Subcommand};
use aetheris_lib::models::Downloader;
use aetheris_lib::ai::{SemanticEngine, DecoderEngine, DessModule};
use aetheris_lib::texture::{bake_and_compress_int8_vector, decompress_and_unpack_int8_vector};
use std::path::{Path, PathBuf};
use std::net::UdpSocket;
use aetheris_lib::proto::root_as_semantic_packet;

#[derive(Parser)]
#[command(name = "babylon", about = "Aetheris Semantic Protocol (ASP) CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize MarianMT models
    Init,
    /// Local translation test: Text -> Vector -> Decode
    Translate { 
        text: String, 
        #[arg(short, long, default_value = "en-ru")]
        direction: String,
        #[arg(short, long, default_value_t = 1337)]
        seed: u64 
    },
    /// Server node: Receive INT8 KTX2 Zstd semantic packets, unshuffle DESS & decode
    Listen { 
        #[arg(short, long, default_value = "127.0.0.1:4433")]
        addr: String,
    },
    /// Client node: Quantize INT8 + Bake KTX2 + Zstd Compress + DESS Shuffle & Send
    Connect { 
        #[arg(short, long)]
        addr: String,
        #[arg(short, long)]
        text: String,
        #[arg(short, long, default_value = "ru")]
        lang: String,
        #[arg(short, long, default_value_t = 1337)]
        seed: u64
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("🏗️  Downloading MarianMT models from Hugging Face...");
            match Downloader::fetch_all() {
                Ok(_) => println!("🚀 Models successfully downloaded and ready."),
                Err(e) => eprintln!("❌ Model download failed: {}", e),
            }
        }

        Commands::Translate { text, direction, seed } => {
            let model_path = PathBuf::from(format!("models/marian-{}", direction));
            if !model_path.exists() {
                eprintln!("❌ Model directory {:?} not found. Please run `babylon init` first.", model_path);
                return Ok(());
            }
            
            let start_load = std::time::Instant::now();
            let mut encoder = SemanticEngine::load(&model_path)?;
            let mut decoder = DecoderEngine::new(&model_path)?;
            println!("🧠 Models loaded in {:?}", start_load.elapsed());
            
            println!("🛰️  Encoding text: '{}'", text);
            let start_encode = std::time::Instant::now();
            let (vector, seq_len, d_model) = encoder.encode(&text)?;
            println!("📊 Hidden states: {} tokens x {} dim. Encoding time: {:?}", seq_len, d_model, start_encode.elapsed());
            
            println!("🔐 Applying DESS Shuffle (Seed: {})", seed);
            let mut shuffled = vector.clone();
            let dess = DessModule::new(seed);
            dess.shuffle(&mut shuffled);
            
            println!("🔓 Unshuffling DESS");
            dess.unshuffle(&mut shuffled);
            
            println!("💬 Decoding semantic vector...");
            let start_decode = std::time::Instant::now();
            let translation = decoder.decode(&shuffled, seq_len, d_model)?;
            println!("⚡ Decoding time: {:?}", start_decode.elapsed());
            println!("👉 Translation: '{}'", translation);
        }

        Commands::Listen { addr } => {
            println!("📡 Aetheris Semantic Protocol (ASP) Server running on UDP {}", addr);
            let socket = UdpSocket::bind(&addr)?;
            let mut buf = [0u8; 65535];
            
            let mut decoders: std::collections::HashMap<String, DecoderEngine> = std::collections::HashMap::new();
            
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((size, src)) => {
                        let receive_time = std::time::Instant::now();
                        println!("📡 [UDP RECEIVE] Packet size: {} bytes from {}", size, src);
                        
                        let packet = match root_as_semantic_packet(&buf[..size]) {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("❌ FlatBuffers parse error: {:?}", e);
                                continue;
                            }
                        };
                        
                        let seed = packet.ghost_hash();
                        let seq_len = packet.sequence_length() as usize;
                        let d_model = packet.hidden_dimension() as usize;
                        let lang_hint = packet.language_hint().unwrap_or("ru");
                        let quant_scale = packet.quant_scale();
                        let quant_min = packet.quant_min();

                        let (scale, min_val) = if quant_scale != 0.0 {
                            (quant_scale, quant_min)
                        } else {
                            (0.02, -2.5)
                        };
                        
                        if let Some(data) = packet.embedding_data() {
                            let compressed_payload = data.bytes();
                            
                            // 1. Zstd Decompress + KTX2 RGBA Unpack + INT8 De-quantize
                            let (mut vector, r_seq, r_dim) = match decompress_and_unpack_int8_vector(compressed_payload, scale, min_val) {
                                Ok(res) => res,
                                Err(_) => {
                                    // Raw float fallback if raw packet
                                    let floats: Vec<f32> = compressed_payload
                                        .chunks_exact(4)
                                        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                                        .collect();
                                    (floats, seq_len, d_model)
                                }
                            };
                            
                            // 2. DESS Unshuffle
                            let dess = DessModule::new(seed);
                            dess.unshuffle(&mut vector);
                            
                            let model_dir = if lang_hint == "ru" {
                                "models/marian-en-ru"
                            } else {
                                "models/marian-ru-en"
                            };
                            
                            let decoder = if let Some(d) = decoders.get_mut(lang_hint) {
                                d
                            } else {
                                println!("🧠 Lazy loading decoder for language {} ({:?})...", lang_hint, model_dir);
                                match DecoderEngine::new(Path::new(model_dir)) {
                                    Ok(d) => {
                                        decoders.insert(lang_hint.to_string(), d);
                                        decoders.get_mut(lang_hint).unwrap()
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Failed to load decoder: {}", e);
                                        continue;
                                    }
                                }
                            };
                            
                            let start_decode = std::time::Instant::now();
                            match decoder.decode(&vector, r_seq, r_dim) {
                                Ok(text) => {
                                    let total_latency = receive_time.elapsed();
                                    println!("👉 [DECODED SEMANTIC TEXT] ({}) from {}: '{}' (Decode: {:?}, E2E Latency: {:?})",
                                        lang_hint, src, text, start_decode.elapsed(), total_latency);
                                }
                                Err(e) => {
                                    eprintln!("❌ Decoding error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("❌ Socket error: {}", e),
                }
            }
        }

        Commands::Connect { addr, text, lang, seed } => {
            let model_dir = if lang == "ru" {
                "models/marian-en-ru"
            } else {
                "models/marian-ru-en"
            };
            
            let model_path = PathBuf::from(model_dir);
            if !model_path.exists() {
                eprintln!("❌ Model path {:?} not found. Please run `babylon init` first.", model_path);
                return Ok(());
            }
            
            let mut encoder = SemanticEngine::load(&model_path)?;
            println!("🛰️  Encoding text: '{}'", text);
            let start_enc = std::time::Instant::now();
            let (mut vector, seq_len, d_model) = encoder.encode(&text)?;
            let raw_fp32_bytes = vector.len() * 4;
            
            println!("🔐 Applying DESS Shuffle (Seed: {})", seed);
            let dess = DessModule::new(seed);
            dess.shuffle(&mut vector);
            
            println!("📦 Baking INT8 KTX2 RGBA Texture & Zstd Compress...");
            let start_bake = std::time::Instant::now();
            let baked = bake_and_compress_int8_vector(&vector, seq_len, d_model, 3)?;
            let hash_name = format!("{}.ktx2.zst", &baked.blake3_hash[..16]);
            println!("   ✅ Baked Payload: {} (Compressed Size: {} B vs Raw FP32: {} B, {:.1}x reduction in {:?})",
                hash_name, baked.compressed_byte_size, raw_fp32_bytes,
                raw_fp32_bytes as f64 / baked.compressed_byte_size as f64,
                start_bake.elapsed());
            
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let language_hint_offset = builder.create_string(&lang);
            let embedding_data_offset = builder.create_vector(&baked.compressed_bytes);
            
            let packet = aetheris_lib::proto::SemanticPacket::create(
                &mut builder,
                &aetheris_lib::proto::SemanticPacketArgs {
                    session_id: 0x42,
                    sequence_id: 1,
                    ghost_hash: seed,
                    precision: aetheris_lib::proto::Precision::INT8,
                    sequence_length: seq_len as u32,
                    hidden_dimension: d_model as u32,
                    embedding_data: Some(embedding_data_offset),
                    quant_scale: baked.quant_scale,
                    quant_min: baked.quant_min,
                    language_hint: Some(language_hint_offset),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                }
            );
            
            builder.finish(packet, None);
            let finished_data = builder.finished_data();
            
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            socket.send_to(finished_data, &addr)?;
            
            println!("🚀 ASP Packet ({}) (Total Wire Size: {} B) sent to {} (Total Encoding+Baking: {:?})!",
                hash_name, finished_data.len(), addr, start_enc.elapsed());
        }
    }
    Ok(())
}

