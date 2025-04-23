FROM rust:slim AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libopenblas-dev liblapack-dev g++ build-essential && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the entire project
COPY . .

# Build the project
RUN cargo build --release

# Create a smaller runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y openssl ca-certificates libopenblas0 liblapack3 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/oort-ml-rust /app/oort-ml-rust

# Expose the port
EXPOSE 8000

# Command to run the executable
CMD ["./oort-ml-rust"]
