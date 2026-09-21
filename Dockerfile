# Multi-stage Docker build for THADAM (Trajectory-aware Heuristics for Autonomous Decentralized AMR Mesh)
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/engine
COPY engine/Cargo.toml engine/Cargo.lock ./
COPY engine/src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/engine/target/release/sih26123 /usr/local/bin/sih26123

EXPOSE 3000

ENV RUST_LOG=info

ENTRYPOINT ["sih26123"]
CMD ["dashboard", "--port", "3000", "--robots", "4", "--tasks", "8"]
