import time
import argparse
from aetheris_lib import Downloader, translate

# List of target language codes (must match SupportedLang enum)
TARGET_LANGS = [
    "uk", "ja", "ko", "es", "de", "zh", "ar", "en", "ru"
]

def benchmark(iterations: int = 10, save: bool = False):
    """Run translation benchmark for each target language.

    Args:
        iterations: Number of times to translate the test sentence per language.
        save: If True, write the markdown table to `benchmark_results.md`.
    """
    sentence = "The quick brown fox jumps over the lazy dog."
    results = []
    for lang in TARGET_LANGS:
        # Warm‑up (some models lazy‑load on first call)
        _ = translate(sentence, lang)
        start = time.perf_counter()
        for _ in range(iterations):
            _ = translate(sentence, lang)
        elapsed = (time.perf_counter() - start) / iterations * 1000  # ms
        results.append((lang, f"{elapsed:.2f}"))

    # Build markdown table
    table_lines = ["| Language | Avg latency (ms) |", "|----------|------------------|"]
    for lang, latency in results:
        table_lines.append(f"| {lang} | {latency} |")
    table = "\n".join(table_lines)
    print(table)
    if save:
        with open("benchmark_results.md", "w", encoding="utf-8") as f:
            f.write(table)
        print("Saved results to benchmark_results.md")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="ASP Babylon translation benchmark")
    parser.add_argument("--iterations", type=int, default=10, help="Iterations per language")
    parser.add_argument("--save", action="store_true", help="Save markdown table to file")
    args = parser.parse_args()
    # Ensure models are downloaded
    Downloader.fetch_all()
    benchmark(iterations=args.iterations, save=args.save)
