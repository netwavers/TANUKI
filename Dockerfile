# Build stage
FROM rust:1.90-slim-bookworm AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    sqlite3 \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/tanuki

# Copy everything
COPY . .

# Build
RUN cargo build --release -p tanuki-serving

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    sqlite3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/tanuki/target/release/tanuki-serving /app/tanuki-serving

# Default port
EXPOSE 3000

# Entry point
CMD ["./tanuki-serving"]
