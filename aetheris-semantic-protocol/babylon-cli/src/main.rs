use clap::{Parser, Subcommand};
use aetheris_lib::models::Downloader;
use aetheris_lib::ai::{SemanticEngine, DecoderEngine, DessModule};
use std::path::{Path, PathBuf};
use std::net::UdpSocket;
use aetheris_lib::proto::root_as_semantic_packet;

#[derive(Parser)]
#[command(name = "babylon", about = "Aetheris Semantic Protocol CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Инициализация ИИ-моделей (MarianMT en-ru и ru-en)
    Init,
    /// Локальный тест: Текст -> Вектор -> Декодирование
    Translate { 
        text: String, 
        /// Языковое направление: "en-ru" или "ru-en"
        #[arg(short, long, default_value = "en-ru")]
        direction: String,
        #[arg(short, long, default_value_t = 1337)]
        seed: u64 
    },
    /// Сервер: Прием семантических векторов, дешифровка и локальный перевод
    Listen { 
        #[arg(short, long, default_value = "127.0.0.1:4433")]
        addr: String,
    },
    /// Клиент: Отправка смысла на удаленный узел
    Connect { 
        #[arg(short, long)]
        addr: String,
        #[arg(short, long)]
        text: String,
        /// Язык получателя: "ru" или "en"
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
            println!("🏗️  Загрузка моделей MarianMT из Hugging Face...");
            match Downloader::fetch_all() {
                Ok(_) => println!("🚀 Модели успешно загружены и готовы к работе."),
                Err(e) => eprintln!("❌ Ошибка загрузки моделей: {}", e),
            }
        }

        Commands::Translate { text, direction, seed } => {
            let model_path = PathBuf::from(format!("models/marian-{}", direction));
            if !model_path.exists() {
                eprintln!("❌ Модель не найдена по пути {:?}. Пожалуйста, запустите `init` сначала.", model_path);
                return Ok(());
            }
            
            let start_load = std::time::Instant::now();
            let mut encoder = SemanticEngine::load(&model_path)?;
            let mut decoder = DecoderEngine::new(&model_path)?;
            println!("🧠 Модели загружены за {:?}", start_load.elapsed());
            
            println!("🛰️  Кодирование: '{}'", text);
            let start_encode = std::time::Instant::now();
            let (vector, seq_len, d_model) = encoder.encode(&text)?;
            println!("📊 Векторы скрытых состояний: {} токенов x {} дим. Кодирование: {:?}", seq_len, d_model, start_encode.elapsed());
            
            println!("🔐 Применение DESS (Seed: {})", seed);
            let mut shuffled = vector.clone();
            let dess = DessModule::new(seed);
            dess.shuffle(&mut shuffled);
            
            println!("🔓 Дешифрование DESS");
            dess.unshuffle(&mut shuffled);
            
            println!("💬 Декодирование смысла...");
            let start_decode = std::time::Instant::now();
            let translation = decoder.decode(&shuffled, seq_len, d_model)?;
            println!("⚡ Время декодирования: {:?}", start_decode.elapsed());
            println!("👉 Результат: '{}'", translation);
        }

        Commands::Listen { addr } => {
            println!("📡 Сервер New Babylon запущен на {}", addr);
            let socket = UdpSocket::bind(&addr)?;
            let mut buf = [0u8; 65535];
            
            let mut decoders: std::collections::HashMap<String, DecoderEngine> = std::collections::HashMap::new();
            
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((size, src)) => {
                        println!("📡 Получен пакет ({} байт) от {}", size, src);
                        
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
                        
                        if let Some(data) = packet.embedding_data() {
                            let bytes = data.bytes();
                            let mut vector: Vec<f32> = bytes
                                .chunks_exact(4)
                                .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                                .collect();
                                
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
                                println!("🧠 Ленивая загрузка декодера для {} ({:?})...", lang_hint, model_dir);
                                match DecoderEngine::new(Path::new(model_dir)) {
                                    Ok(d) => {
                                        decoders.insert(lang_hint.to_string(), d);
                                        decoders.get_mut(lang_hint).unwrap()
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Не удалось загрузить декодер: {}", e);
                                        continue;
                                    }
                                }
                            };
                            
                            match decoder.decode(&vector, seq_len, d_model) {
                                Ok(text) => {
                                    println!("👉 [СМЫСЛ РАСШИФРОВАН] ({}) от {}: '{}'", lang_hint, src, text);
                                }
                                Err(e) => {
                                    eprintln!("❌ Ошибка декодирования: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("❌ Ошибка сокета: {}", e),
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
                eprintln!("❌ Модель не найдена по пути {:?}. Пожалуйста, запустите `init` сначала.", model_path);
                return Ok(());
            }
            
            let mut encoder = SemanticEngine::load(&model_path)?;
            println!("🛰️  Кодирование смысла: '{}'", text);
            let (mut vector, seq_len, d_model) = encoder.encode(&text)?;
            
            println!("🔐 Шифрование DESS (Seed: {})", seed);
            let dess = DessModule::new(seed);
            dess.shuffle(&mut vector);
            
            let mut builder = flatbuffers::FlatBufferBuilder::new();
            let language_hint_offset = builder.create_string(&lang);
            
            let byte_vector: Vec<u8> = vector.iter().flat_map(|&f| f.to_le_bytes().to_vec()).collect();
            let embedding_data_offset = builder.create_vector(&byte_vector);
            
            let packet = aetheris_lib::proto::SemanticPacket::create(
                &mut builder,
                &aetheris_lib::proto::SemanticPacketArgs {
                    session_id: 0x42,
                    sequence_id: 1,
                    ghost_hash: seed,
                    precision: aetheris_lib::proto::Precision::F32,
                    sequence_length: seq_len as u32,
                    hidden_dimension: d_model as u32,
                    embedding_data: Some(embedding_data_offset),
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
            
            println!("🚀 Пакет ASP ({} байт) отправлен на {}!", finished_data.len(), addr);
        }
    }
    Ok(())
}
