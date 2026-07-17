FROM rust:1.73 AS builder
WORKDIR /app

# Copy the Cargo manifest and source files
COPY Cargo.toml Cargo.lock .
COPY demo ./demo
COPY aetheris-protocol ./aetheris-protocol
COPY aetheris-lib ./aetheris-lib
COPY benchmarks ./benchmarks
COPY README.md .
COPY docs ./docs

# Build all crates in release mode
RUN cargo build --release --workspace

# Runtime image
FROM debian:buster-slim
WORKDIR /app

# Copy the compiled server binary
COPY --from=builder /app/target/release/asp_demo_server ./asp_demo_server

EXPOSE 8080

CMD ["./asp_demo_server"]
