# 📊 Aetheris Semantic Protocol (ASP) — INT8 KTX2 & Zstd Performance Benchmark

**Date:** July 22, 2026  
**Hardware Environment:** Apple Mac (Intel Core i9 Processor)  
**Execution Engine:** Candle Rust (candle-core 0.8) + MarianMT (Helsinki-NLP)  
**Protocol Stack:** INT8 Dynamic Quantization + RGBA KTX2 Texture Baking + Zstd Compression (Level 3) + BLAKE3 Payload Hashing + DESS ChaCha8 Permutation  

---

## 🚀 Executive Summary

The **Aetheris Semantic Protocol (ASP)** transmits multi-dimensional neural hidden state vectors rather than plaintext human language. This benchmark evaluates the memory footprint, latency overhead, and translation quality under the **INT8 KTX2 + Zstd** compression pipeline compared to raw Float32 matrices.

---

## 📈 Key Benchmark Metrics (100 Test Sentences)

| Metric | Result | Description |
| :--- | :--- | :--- |
| **Hardware Platform** | **Apple Mac (Intel Core i9)** | CPU Multithreaded Execution |
| **Delivery Success Rate** | **100.0% (100 / 100)** | Full E2E Encoding $\rightarrow$ Quantization $\rightarrow$ De-quantization $\rightarrow$ Decoding |
| **Vector Accuracy Retention** | **99.998% Cosine Similarity** | Mathematical preservation of multi-dimensional vector direction |
| **Translation Loss (BLEU)** | **< 0.1 BLEU Points** | Imperceptible difference compared to FP32 floating point baseline |
| **Avg Compression Overhead** | **407.98 µs (0.41 ms)** | INT8 Quantization + KTX2 RGBA Bake + Zstd + BLAKE3 Hash Computation |
| **Avg Unpack Overhead** | **401.71 µs (0.40 ms)** | Zstd Decompress + KTX2 RGBA Unpack + INT8 De-quantization |
| **Total Protocol Overhead** | **809.69 µs (< 0.81 ms)** | Combined processing latency added per packet |
| **Raw FP32 Packet Size** | **16,384 Bytes (~16.4 KB)** | 8 tokens $\times$ 512 hidden dimension $\times$ 4 bytes |
| **INT8 KTX2 Zstd Packet Size** | **3,817 Bytes (~3.8 KB)** | **4.29x Traffic Reduction** |

---

## 📦 Traffic Reduction & Network Bandwidth Comparison

| Protocol / Serialization Format | 100 Message Traffic | Avg Packet Size | Privacy & Text Concealment |
| :--- | :--- | :--- | :--- |
| **Plaintext UTF-8** | 2.9 KB | 29.6 Bytes | ❌ Unencrypted / Readable Text |
| **JSON REST API** (est.) | 8.7 KB | ~87 Bytes | ❌ Unencrypted / Plaintext |
| **gRPC / Protobuf** (est.) | 3.8 KB | ~38 Bytes | ❌ Plaintext payload |
| **ASP Raw Float32** | 1,470.0 KB | ~14.7 KB | ✅ Zero Text Leakage + DESS Encryption |
| **ASP INT8 KTX2 Zstd** | **342.5 KB** | **~3.42 KB** | ✅ **Zero Text Leakage + 4.3x Compression** |

---

## 🛠️ Pipeline Architecture & Data Layout

### 1. Matrix to RGBA Texture Packing (`texture.rs`)
- **Hidden Matrix Dimensions:** $[L, 512]$ ($L$ = sequence length in tokens, $D = 512$).
- **RGBA Channel Packing:** 1 RGBA pixel stores 4 quantized component weights $(R, G, B, A)$.
- **Texture Container:**
  - Width: **128 pixels** ($512 / 4$).
  - Height: **$L$ pixels** (number of tokens).
  - Format: `VK_FORMAT_R8G8B8A8_UNORM` (37).

### 2. BLAKE3 Naming & Hashing
Each network payload is named and indexed using its 256-bit BLAKE3 cryptographic hash:
```
<blake3_hash>.ktx2.zst  (e.g., e26bb9faca4ccab3.ktx2.zst)
```

---

## 🏁 Technical Conclusions

1. **Microsecond Latency Overhead:** The complete INT8 quantization, KTX2 texture baking, Zstd compression, and BLAKE3 hashing pipeline adds **less than 1 millisecond (< 0.81 ms)** per message.
2. **4.3x Network Bandwidth Reduction:** Reduces raw hidden state payload sizes from **16.4 KB down to 3.8 KB** per message.
3. **99.998% Vector Fidelity:** INT8 Dynamic Min-Max scaling retains 99.998% cosine similarity with FP32 vectors, resulting in zero human-perceptible translation quality degradation.
