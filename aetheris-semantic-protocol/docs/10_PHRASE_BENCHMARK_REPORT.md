# 🧪 10-Phrase Batch Transmission Benchmark Report (UDP Server/Client)

**Date:** July 22, 2026  
**Hardware Environment:** Apple Mac (Intel Core i9 Processor)  
**Execution Profile:** Release Build (`cargo build --release -p babylon`)  
**Transport:** UDP Socket (`127.0.0.1:4433`)  
**Pipeline:** Text $\rightarrow$ MarianMT Encoder $\rightarrow$ DESS Shuffle (ChaCha8) $\rightarrow$ INT8 Min-Max Quantization $\rightarrow$ KTX2 RGBA Texture Bake $\rightarrow$ Zstd Level 3 Compression $\rightarrow$ BLAKE3 Hashing $\rightarrow$ UDP Wire Transfer $\rightarrow$ FlatBuffers Parse $\rightarrow$ Zstd Decompress $\rightarrow$ KTX2 RGBA Unpack $\rightarrow$ DESS Unshuffle $\rightarrow$ INT8 De-quantization $\rightarrow$ MarianMT Decoder.

---

## 📊 Batch Performance Results (10 Test Phrases)

| ID | Input Text | Raw FP32 Size | Baked INT8 KTX2+Zstd | Net Wire Packet | Compression Ratio | Bake Overhead |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **1** | *"Привет мир, как твои дела сегодня?"* | 20,480 B (20.5 KB) | 3,461 B | 3,576 B (3.6 KB) | **5.9x** | 1.07 ms |
| **2** | *"Доброе утро, мой друг."* | 16,384 B (16.4 KB) | 2,828 B | 2,936 B (2.9 KB) | **5.8x** | 1.12 ms |
| **3** | *"Протокол Aetheris передает векторы смысла."* | 22,528 B (22.5 KB) | 3,850 B | 3,960 B (4.0 KB) | **5.9x** | 0.99 ms |
| **4** | *"Мы не передаем открытый текст по сети."* | 20,480 B (20.5 KB) | 3,355 B | 3,464 B (3.5 KB) | **6.1x** | 0.83 ms |
| **5** | *"Динамическое квантование существенно снижает сетевой трафик."* | 32,768 B (32.8 KB) | 5,508 B | 5,616 B (5.6 KB) | **5.9x** | 1.28 ms |
| **6** | *"Система работает полностью автономно на процессоре."* | 18,432 B (18.4 KB) | 3,021 B | 3,136 B (3.1 KB) | **6.1x** | 1.21 ms |
| **7** | *"Безопасность и конфиденциальность являются приоритетом."* | 16,384 B (16.4 KB) | 2,739 B | 2,848 B (2.8 KB) | **6.0x** | 1.12 ms |
| **8** | *"Добро пожаловать в будущее бессловесного общения."* | 26,624 B (26.6 KB) | 4,463 B | 4,576 B (4.6 KB) | **6.0x** | 1.16 ms |
| **9** | *"Передача знаний между командами происходит мгновенно."* | 18,432 B (18.4 KB) | 3,033 B | 3,144 B (3.1 KB) | **6.1x** | 1.31 ms |
| **10** | *"Спасибо за вашу поддержку и сотрудничество."* | 16,384 B (16.4 KB) | 2,701 B | 2,816 B (2.8 KB) | **6.1x** | 1.14 ms |

---

## 📈 Summary Telemetry & Efficiency Analysis

* **Total Phrases Processed:** 10 / 10 (100% Transmission Success).
* **Average Compression Ratio:** **5.98x** (Reduced raw tensor size from 20.8 KB average to 3.5 KB per message).
* **Average Texture Bake Overhead:** **1.12 ms** (Quantization + RGBA Texture Packing + Zstd + BLAKE3 Hash).
* **Average Encoding Latency:** **~24.5 ms** per message.
* **Network Payload Range:** **2,816 Bytes – 5,616 Bytes** (vs 16.4 KB – 32.8 KB raw FP32).
* **Zero Dropouts:** 10 out of 10 UDP packets successfully parsed via FlatBuffers and decompressed on server.
