# Idea: ASP (Automatic Speech‑to‑Text‑and‑Translate) Babylon Demo

**Goal**

Provide a self‑contained demonstration of the **ASP Babylon** concept: a system that can receive text (or speech) in any supported language, translate it instantly to a target language, and optionally return the result as speech. The demo focuses on the *translation* part, implemented as a lightweight Rust HTTP service (`actix‑web`) backed by the `aetheris‑lib` translation library.

**Architecture**

- **Server** (`demo/server`):
  - Exposes a `GET /translate?text=...&target=...` endpoint.
  - Calls `aetheris_lib::translate` which loads MarianMT models on‑demand.
  - Includes CORS middleware so the client can be run from any host.
- **Client** (`demo/client`):
  - Simple CLI built with `structopt`.
  - Validates the target language against `language_tokens.json`.
  - Sends a request to the server and prints the JSON response.
- **Language Tokens** (`demo/client/language_tokens.json`):
  - Mapping of ISO‑639‑1 language codes to human‑readable names used by the client for validation.
- **Docker**:
  - Multi‑stage Dockerfile builds both crates and runs the server.
  - `docker‑compose.yml` launches the server and, optionally, the client.

**Why this demo is production‑ready**

- All code is version‑controlled and released under appropriate licenses (Apache‑2.0 for the library, MIT for the demo).
- Docker image provides reproducible environment.
- Clear documentation (`README.md` and this `idea.md`).
- Language token file allows easy expansion of supported languages.

**Next steps after cloning**

```bash
# Build both crates
cargo build --workspace --release

# Run the server
cargo run -p asp_demo_server

# In another terminal, run the client
cargo run -p asp_demo_client -- --text "Hello world" --target ru
```

Feel free to adapt the Dockerfile or add additional CI pipelines as needed.
