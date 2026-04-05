FROM rust:1.86 AS builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release -p api_server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api_server /usr/local/bin/api_server

ENV LISTEN_ADDR=0.0.0.0:3000

EXPOSE 3000

CMD ["api_server"]
