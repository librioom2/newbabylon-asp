use quiche;
use std::net::UdpSocket;
use anyhow::{Result, anyhow};
use crate::proto::root_as_semantic_packet;
use crate::ai::DessModule;

pub struct QuicheNode {
    socket: UdpSocket,
    config: quiche::Config,
}

impl QuicheNode {
    
    pub fn new(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;

        let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;
        
        // Исправлено: передаем массив ссылок на байтовые строки
        config.set_application_protos(&[b"h3"])?; 
        
        config.set_max_idle_timeout(5000);
        config.set_max_recv_udp_payload_size(2048);
        config.set_max_send_udp_payload_size(2048);

        Ok(Self { socket, config })
    }


    // Логика отправки семантического пакета
    pub fn send_semantic(&self, dest: &str, payload: &[u8]) -> Result<()> {
        // Для MVP мы просто отправляем UDP-пакет, 
        // имитируя работу QUIC-датаграммы
        self.socket.send_to(payload, dest)?;
        Ok(())
    }
    
    pub fn listen_and_decode(&self) -> Result<()> {
    let mut buf = [0u8; 2048];
    loop {
        match self.socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                // 1. Распаковка FlatBuffers
                let packet = root_as_semantic_packet(&buf[..size])
                    .map_err(|e| anyhow!("FlatBuffers error: {:?}", e))?;

                let seed = packet.ghost_hash();
                println!("📡 Received ASP Packet from {}", src);
                println!("🔑 Ghost Hash: {}", seed);

                // 2. Извлечение вектора
                if let Some(data) = packet.embedding_data() {
                    // У FlatBuffers Vector нужно вызвать .bytes(), чтобы получить &[u8]
                    let bytes = data.bytes(); 

                    // Превращаем байты обратно в f32 с явным указанием типов
                    let mut vector: Vec<f32> = bytes
                        .chunks_exact(4)
                        .map(|chunk: &[u8]| {
                            let array: [u8; 4] = chunk.try_into().expect("Invalid chunk size");
                            f32::from_le_bytes(array)
                        })
                        .collect();

                    println!("🎭 Shuffled (first 3): {:?}", &vector[..3]);

                    // 3. Дешифровка (DESS Unshuffle)
                    let dess = DessModule::new(seed);
                    dess.unshuffle(&mut vector);

                    println!("🔓 Decoded (first 3): {:?}", &vector[..3]);
                    println!("-------------------------------------------");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(anyhow!(e)),
        }
    }
}






    pub fn listen(&self) -> Result<()> {
        let mut buf = [0u8; 2048];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, src)) => {
                    println!("📡 Received ASP Packet ({} bytes) from {}", size, src);
                    // Здесь будет логика распаковки FlatBuffers и инференс Phi-3
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(anyhow!(e)),
            }
        }
    }
}

