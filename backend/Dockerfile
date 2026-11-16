FROM rust:1.100-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/btc-futures-ai-lab-backend /usr/local/bin/btc-futures-ai-lab-backend
EXPOSE 8080
CMD ["/usr/local/bin/btc-futures-ai-lab-backend"]
