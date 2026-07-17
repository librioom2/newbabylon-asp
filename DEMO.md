# Demo Instructions

## Running the Server
```bash
# Build the project (if not already built)
cargo build --workspace --release

# Run the server
cargo run -p asp_demo_server
```
The server will start on `http://127.0.0.1:8080` and expose the `/translate` endpoint.

## Running the Client
```bash
# In a separate terminal, run the client
cargo run -p asp_demo_client -- --text "Hello world" --target ru
```
The client validates the target language using `language_tokens.json`, sends a request to the server, and prints the translation.

## Docker
### Build and Run
```bash
# Build the Docker image
docker build -t asp_demo .

# Run the container
docker run -p 8080:8080 asp_demo
```
The server will be accessible at `http://localhost:8080`.

## Language Tokens
Supported languages are defined in `demo/client/language_tokens.json`. Feel free to extend this file with additional language codes.

---
Feel free to explore, modify, and extend the demo as needed!
