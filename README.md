# ASP Babylon Demo

## Overview
This repository showcases the **ASP (Babylon) protocol** for instant, high‑quality multi‑language translation. The demo includes:
- A Rust library (`aetheris-lib`) that downloads MarianMT models and provides a simple translation API.
- Python scripts that use the library via the generated bindings.
- A **console benchmark** that measures translation latency for a set of target languages.
- Docker support for one‑click execution.

## Features
- **Real‑time translation** performed locally without calling external APIs.
- **Error reduction** thanks to a unified tokenizer and model set.
- **Barrier removal** – users can communicate in their native language while messages are automatically translated.
- **Future‑ready** – binary blobs are already supported, making STT/TTS integration straightforward.

## Quick‑Start
```bash
# Clone the repository
git clone https://github.com/your-org/asp-babylon-demo.git
cd asp-babylon-demo

# Install Rust toolchain (required for aetheris-lib)
rustup toolchain install stable

# Build the Rust library
cargo build --release

# Install Python dependencies
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt

# Download all models (includes en, ru, de, fr, es, zh, ar, uk, ja, ko)
python -c "from aetheris_lib import Downloader; Downloader.fetch_all()"

# Run a simple translation (English → Russian)
python -c "from aetheris_lib import translate; print(translate('The quick brown fox jumps over the lazy dog.', 'ru'))"
```

## Benchmark
Run the benchmark script to see average latency per language:
```bash
python benchmarks/translate_benchmark.py
```
The script prints a markdown table like:
```
| Language | Avg latency (ms) |
|----------|------------------|
| ru       | 45.2 |
| de       | 48.6 |
| ...      | ... |
```
The benchmark can also be executed inside the provided Docker container:
```bash
docker build -t asp-benchmark -f Dockerfile .
docker run --rm asp-benchmark
```

## Sponsorship
We are looking for sponsors to support the continuation of this project (hosting, model storage, future STT/TTS integration). Please see the **One‑Pager** in the `docs/` folder for details.

- **GitHub Sponsors:** https://github.com/sponsors/your-org
- **Open Collective:** https://opencollective.com/your-org

---
## License
- Core library (`aetheris-lib`): Apache‑2.0
- Demo scripts and benchmark: MIT
- Documentation and presentation assets: CC‑BY‑4.0
