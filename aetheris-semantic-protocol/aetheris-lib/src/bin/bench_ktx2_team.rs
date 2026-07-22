// bench_ktx2_team.rs — EN-Pivot Team Benchmark with KTX2 RGBA Texture & Zstd Compression
use anyhow::{anyhow, Result};
use aetheris_lib::ai::{SemanticEngine, DecoderEngine, DessModule};
use aetheris_lib::texture::{bake_and_compress_vector, decompress_and_unpack_vector};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() -> Result<()> {
    println!("═════════════════════════════════════════════════════════════════════════");
    println!("  🚀 BENCHMARK: EN-Pivot Team Architecture + KTX2/Zstd Vector Compression");
    println!("═════════════════════════════════════════════════════════════════════════\n");

    let phrases_raw = std::fs::read_to_string("phrases_100.json")
        .or_else(|_| std::fs::read_to_string("aetheris-semantic-protocol/phrases_100.json"))?;
    let phrases: Vec<String> = serde_json::from_str(&phrases_raw)?;

    // Team Model Paths (Separated Encoders & Decoders)
    // Member A (RU speaker): Encoder ru-en, Decoder en-ru
    // Member C (DE speaker): Encoder de-en, Decoder en-de
    let ru_encoder_dir = get_model_path("models/marian-ru-en");
    let de_decoder_dir = get_model_path("models/marian-en-de");

    if !ru_encoder_dir.exists() || !de_decoder_dir.exists() {
        eprintln!("⚠️  Models not found. Required: {:?} and {:?}", ru_encoder_dir, de_decoder_dir);
        eprintln!("    Falling back to standard en-de pipeline for matrix compression benchmarks...");
    }

    // Load available encoder & decoder
    let enc_path = if ru_encoder_dir.exists() { ru_encoder_dir } else { get_model_path("models/marian-en-de") };
    let dec_path = de_decoder_dir;

    println!("🔄 [MEMBER A] Loading Encoder from {:?}...", enc_path);
    let mut encoder = SemanticEngine::load(&enc_path)?;

    println!("🔄 [MEMBER C] Loading Decoder from {:?}...", dec_path);
    let mut decoder = DecoderEngine::new(&dec_path)?;
    println!("✅ Team models ready.\n");

    let seed: u64 = 9999;
    let dess = DessModule::new(seed);

    let mut total_raw_bytes: usize = 0;
    let mut total_ktx2_bytes: usize = 0;
    let mut total_zstd_bytes: usize = 0;

    let mut bake_times: Vec<std::time::Duration> = Vec::new();
    let mut unpack_times: Vec<std::time::Duration> = Vec::new();
    let mut sample_results: Vec<(String, String, String, usize, usize, f64)> = Vec::new();

    let start_all = Instant::now();

    println!("┌────┬──────────────────────────────────────┬──────────┬──────────┬──────────┬────────┐");
    println!("│  # │ Phrase (English / Input)             │ F32 Raw  │ KTX2 Raw │ Zstd Pack│ Ratio  │");
    println!("├────┼──────────────────────────────────────┼──────────┼──────────┼──────────┼────────┤");

    for (i, phrase) in phrases.iter().enumerate() {
        // 1. Encode
        let (mut vector, seq_len, d_model) = encoder.encode(phrase)?;
        let raw_f32_bytes = vector.len() * 4;

        // 2. DESS Shuffle
        dess.shuffle(&mut vector);

        // 3. Bake KTX2 + Zstd + BLAKE3
        let t_bake_start = Instant::now();
        let baked = bake_and_compress_vector(&vector, seq_len, d_model, 3)?;
        bake_times.push(t_bake_start.elapsed());

        total_raw_bytes += raw_f32_bytes;
        total_ktx2_bytes += baked.original_byte_size;
        total_zstd_bytes += baked.compressed_byte_size;

        let ratio = raw_f32_bytes as f64 / baked.compressed_byte_size as f64;

        if (i + 1) % 10 == 0 || i == 0 {
            let trunc: String = phrase.chars().take(34).collect();
            println!("│{:>3} │ {:<36} │ {:>6} B │ {:>6} B │ {:>6} B │ {:>5.1}x │",
                i + 1,
                if phrase.chars().count() > 34 { format!("{}…", trunc) } else { trunc },
                raw_f32_bytes,
                baked.original_byte_size,
                baked.compressed_byte_size,
                ratio
            );
        }

        // 4. Transport Payload Simulation (<blake3_hash>.ktx2.zst)
        let payload_hash = baked.blake3_hash.clone();
        let compressed_payload = baked.compressed_bytes;

        // 5. Member C: Decompress Zstd -> Unpack KTX2 -> Unshuffle DESS -> Decode
        let t_unpack_start = Instant::now();
        let (mut restored_vector, r_seq, r_dim) = decompress_and_unpack_vector(&compressed_payload)?;
        unpack_times.push(t_unpack_start.elapsed());

        dess.unshuffle(&mut restored_vector);

        let translation = decoder.decode(&restored_vector, r_seq, r_dim)?;

        if (i + 1) % 20 == 0 || i == 0 {
            sample_results.push((
                phrase.clone(),
                translation,
                payload_hash[..16].to_string(),
                raw_f32_bytes,
                baked.compressed_byte_size,
                ratio,
            ));
        }
    }

    println!("└────┴──────────────────────────────────────┴──────────┴──────────┴──────────┴────────┘");

    let total_elapsed = start_all.elapsed();
    let avg_bake = bake_times.iter().sum::<std::time::Duration>() / phrases.len() as u32;
    let avg_unpack = unpack_times.iter().sum::<std::time::Duration>() / phrases.len() as u32;
    let total_compression_ratio = total_raw_bytes as f64 / total_zstd_bytes as f64;

    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║          📦 SUMMARY: KTX2 + ZSTD VECTOR MATRIX COMPRESSION           ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║                                                                       ║");
    println!("║  📊 TRAFFIC REDUCTION                                                 ║");
    println!("║  ─────────────────────────────────────────────────────                ║");
    println!("║  Raw FP32 Matrix (100 msgs):   {:>8} bytes ({:>6.1} KB)              ║",
        total_raw_bytes, total_raw_bytes as f64 / 1024.0);
    println!("║  KTX2 RGBA Texture:            {:>8} bytes ({:>6.1} KB)              ║",
        total_ktx2_bytes, total_ktx2_bytes as f64 / 1024.0);
    println!("║  KTX2 + Zstd Payload:          {:>8} bytes ({:>6.1} KB)              ║",
        total_zstd_bytes, total_zstd_bytes as f64 / 1024.0);
    println!("║  Overall Compression Ratio:    {:>6.2}x savings                       ║", total_compression_ratio);
    println!("║                                                                       ║");
    println!("║  ⏱️  OVERHEAD LATENCY                                                  ║");
    println!("║  ─────────────────────────────────────────────────────                ║");
    println!("║  Avg KTX2 Bake + Zstd + BLAKE3: {:>7.2?}                               ║", avg_bake);
    println!("║  Avg Zstd Unpack + KTX2 Decode: {:>7.2?}                               ║", avg_unpack);
    println!("║  Total Processing Overhead:     {:>7.2?} / message                     ║", avg_bake + avg_unpack);
    println!("║  Total Wall Clock Time:         {:>7.2?}                               ║", total_elapsed);
    println!("║                                                                       ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    println!("\n📝 Sample Team Translations & Payload Hashing:");
    for (idx, (en, de, hash, raw_b, zstd_b, r)) in sample_results.iter().enumerate() {
        println!("  {}. Payload: {}.ktx2.zst ({} B -> {} B, {:.1}x)", idx + 1, hash, raw_b, zstd_b, r);
        println!("     [EN]: \"{}\"", en);
        println!("     [DE]: \"{}\"", de);
        println!();
    }

    Ok(())
}

fn get_model_path(relative: &str) -> PathBuf {
    let p = PathBuf::from(relative);
    if p.exists() {
        p
    } else {
        PathBuf::from(format!("aetheris-semantic-protocol/{}", relative))
    }
}
