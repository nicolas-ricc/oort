FROM rust:slim AS builder

WORKDIR /usr/src/app

# Install build dependencies and cargo-watch
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libopenblas-dev liblapack-dev g++ build-essential curl && \
    cargo install cargo-watch && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock* ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build && \
    rm -rf src

# Copy the actual source code
COPY . .

# Copy healthcheck script
COPY preload_models.sh /usr/src/app/preload_models.sh
RUN chmod +x /usr/src/app/preload_models.sh

# Expose the port
EXPOSE 8000
ENV DB_NODES="oort-db:9042"
# Command to run with hot-reloading
CMD ["cargo", "watch", "-x", "run", "-w", "src"]