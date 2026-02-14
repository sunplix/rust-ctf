FROM rust:1.84-bookworm AS builder
WORKDIR /app

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rust-ctf-backend /usr/local/bin/rust-ctf-backend

EXPOSE 8080
CMD ["rust-ctf-backend"]
