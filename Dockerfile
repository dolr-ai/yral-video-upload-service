# Build stage
FROM rust:1.85 as builder

WORKDIR /app

# Copy manifest files
COPY Cargo.toml Cargo.lock* ./
COPY rust-toolchain.toml ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary from builder stage
COPY --from=builder /app/target/release/yral-video-upload-service /app/yral-video-upload-service

# Set environment variables
ENV RUST_LOG=info

# Expose the service port
EXPOSE 3000

# Run the service
CMD ["/app/yral-video-upload-service"]
