FROM rust:1.75-slim-bookworm as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/kora-reclaim-rs /usr/local/bin/koralreef

WORKDIR /data
ENTRYPOINT ["koralreef"]
CMD ["--config", "/data/config.toml"]
