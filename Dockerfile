# Multi-stage build for Rust REST API with ClickHouse
# Stage 1: Build stage
FROM rustlang/rust:nightly-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy Cargo files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to cache dependencies
RUN mkdir -p src/bin && echo "fn main() {println!(\"dummy\")}" > src/main.rs && echo "fn main() {println!(\"dummy\")}" > src/bin/migrate.rs

# Build dependencies (this layer will be cached unless Cargo files change)
RUN cargo build --release
RUN rm src/main.rs src/bin/migrate.rs

# Copy source code
COPY src ./src

# Build the actual application
# Remove the dummy targets first
RUN rm -f target/release/deps/rest_api_rust_clickhouse*
RUN rm -f target/release/deps/migrate*
RUN cargo build --release
RUN cargo build --release --bin migrate

# Stage 2: Runtime stage
FROM debian:sid-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user for security
RUN useradd --create-home --shell /bin/bash --user-group --uid 1000 appuser

# Create app directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /usr/src/app/target/release/rest-api-rust-clickhouse /app/rest-api-rust-clickhouse
COPY --from=builder /usr/src/app/target/release/migrate /app/migrate

# Change ownership to app user
RUN chown appuser:appuser /app/rest-api-rust-clickhouse /app/migrate
RUN chmod +x /app/rest-api-rust-clickhouse /app/migrate

# Switch to non-root user
USER appuser

# Expose port (default port for web servers)
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=3000

# Run the application
CMD ["/app/rest-api-rust-clickhouse"]
