# Multi-stage Dockerfile for Rust Global Chat
# Stage 1: Build
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/app

# Copy dependency definitions
COPY Cargo.toml Cargo.lock ./

# Build dummy target to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build release binary
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim AS runner

WORKDIR /app

# Install ca-certificates (required for TLS to Neon/Supabase cloud databases)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/global_chat /app/global_chat

# Copy static frontend directory
COPY static ./static

ENV PORT=3000
EXPOSE 3000

CMD ["/app/global_chat"]
