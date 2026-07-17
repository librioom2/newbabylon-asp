# 🏛️ New Babylon — Semantic Protocol (ASP)

**Aetheris Semantic Protocol** — протокол передачи смысла для интернациональных команд.

Отправитель кодирует текст в **вектор скрытых состояний** (encoder MarianMT), шифрует его через **DESS** (Dynamic Embedding Space Shuffling), передаёт по сети через **quiche** (QUIC / UDP) в формате **FlatBuffers** (`SemanticPacket`). Получатель дешифрует вектор и декодирует его на **свой язык** (decoder MarianMT).

> Не текст передаётся по сети — передаётся **смысл**.

## Архитектура

```
 Клиент (Отправитель)                         Сервер (Получатель)
┌────────────────────┐                    ┌────────────────────┐
│  Текст (en)        │                    │  Получен SemanticPkt│
│        ↓           │                    │        ↓           │
│  MarianMT Encoder  │                    │  DESS Unshuffle    │
│        ↓           │                    │        ↓           │
│  DESS Shuffle      │   ── QUIC/UDP ──▸  │  MarianMT Decoder  │
│        ↓           │   FlatBuffers      │        ↓           │
│  SemanticPacket    │                    │  Текст (ru/de/…)   │
└────────────────────┘                    └────────────────────┘
```

## Стек

| Слой | Технология | Назначение |
|------|-----------|------------|
| AI Engine | **Candle** (candle-core, candle-transformers, candle-nn) | Инференс MarianMT на CPU |
| Модели | **Helsinki-NLP/opus-mt** (MarianMT Seq2Seq) | Кодирование / декодирование смысла |
| Сериализация | **FlatBuffers** | Zero-copy формат для SemanticPacket |
| Транспорт | **quiche** (QUIC over UDP, HTTP/3) | Быстрая доставка пакетов |
| Шифрование | **DESS** (ChaCha8 + vector shuffle) | Обфускация вектора |
| Токенизация | **tokenizers** (HuggingFace) | BPE-токенизация текста |

## Quick Start

```bash
# 1. Клонируем
git clone --recursive https://github.com/librioom2/newbabylon-asp.git
cd newbabylon-asp/aetheris-protocol

# 2. Собираем
cargo build --release

# 3. Скачиваем модели (en-ru, ru-en, en-de, en-fr, en-es, en-zh, en-ar, en-uk, en-ja, en-ko)
cargo run --release -p babylon -- init

# 4. Локальный тест перевода
cargo run --release -p babylon -- translate "Hello world" --direction en-ru
```

## Сетевой режим (server / client)

### Терминал 1 — Сервер (получатель)
```bash
cargo run --release -p babylon -- listen --addr 127.0.0.1:4433
```

### Терминал 2 — Клиент (отправитель)
```bash
cargo run --release -p babylon -- connect \
  --addr 127.0.0.1:4433 \
  --text "The quick brown fox jumps over the lazy dog" \
  --lang ru \
  --seed 1337
```

Клиент кодирует текст → шифрует DESS → упаковывает в FlatBuffers SemanticPacket → отправляет через UDP.
Сервер принимает → дешифрует DESS → декодирует MarianMT → выводит перевод.

## Поддерживаемые языки

| Код | Язык |
|-----|------|
| `en` | English |
| `ru` | Русский |
| `de` | Deutsch |
| `fr` | Français |
| `es` | Español |
| `zh` | 中文 |
| `ar` | العربية |
| `uk` | Українська |
| `ja` | 日本語 |
| `ko` | 한국어 |

## FlatBuffers Schema (SemanticPacket)

```fbs
namespace Babylon;

enum Precision : byte { F32 = 0, F16 = 1, INT8 = 2 }

table SemanticPacket {
  session_id: ulong;
  sequence_id: uint;
  ghost_hash: ulong;           // DESS seed
  precision: Precision;
  sequence_length: uint;        // L
  hidden_dimension: uint;       // D
  embedding_data: [ubyte];      // L * D * sizeof(precision)
  language_hint: string;        // Target language (ru, en, ja…)
  timestamp: ulong;
}
```

## Структура проекта

```
NewBabylon/
├── aetheris-protocol/          # ⚙️ Основной Rust workspace (submodule)
│   ├── aetheris-lib/           # Core library
│   │   ├── src/
│   │   │   ├── ai/mod.rs       # SemanticEngine, DecoderEngine, DESS
│   │   │   ├── transport/mod.rs# QuicheNode (QUIC / UDP)
│   │   │   ├── proto/          # FlatBuffers schema + generated code
│   │   │   ├── models.rs       # Downloader (HuggingFace models)
│   │   │   └── lib.rs
│   │   └── build.rs            # Quiche / BoringSSL build
│   ├── babylon-cli/            # CLI: init, translate, listen, connect
│   ├── models/                 # Downloaded model weights (gitignored)
│   └── Cargo.toml              # Workspace config
├── docs/
│   └── idea.md                 # Concept document
├── certs/                      # TLS certificates for quiche
│   └── generate.sh
└── README.md
```

## TLS-сертификаты (для quiche)

```bash
cd certs && bash generate.sh
```

## Sponsorship

We are looking for sponsors to support the continuation of this project (hosting, model storage, future STT/TTS integration). Please see the **One‑Pager** in the `docs/` folder for details.

- **GitHub Sponsors:** https://github.com/sponsors/your-org
- **Open Collective:** https://opencollective.com/your-org

---

## License
- Core library (`aetheris-lib`): Apache‑2.0
- CLI and demo scripts: MIT
- Documentation and presentation assets: CC‑BY‑4.0
