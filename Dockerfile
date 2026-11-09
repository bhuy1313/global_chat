# Stage 1: Build binary using the latest official Rust image
FROM rust:bookworm AS builder

WORKDIR /usr/src/app

# Copy dependency manifests and source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the release binary
RUN cargo build --release

# Stage 2: Runtime image (ultra-lightweight Debian slim)
FROM debian:bookworm-slim AS runner

WORKDIR /app

# Install ca-certificates for secure TLS connections to Cloud DB
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from builder stage
COPY --from=builder /usr/src/app/target/release/global_chat /app/global_chat

# Copy static assets (HTML/CSS/JS)
COPY static ./static

ENV PORT=3000
EXPOSE 3000

CMD ["/app/global_chat"]
