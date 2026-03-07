FROM nvidia/cuda:12.4.1-devel-ubuntu22.04 AS builder

WORKDIR /usr/src/app

# Install Rust and build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libopenblas-dev liblapack-dev g++ build-essential curl && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . /root/.cargo/env && \
    cargo install cargo-watch && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# CUDA compute capability: 89 = RTX 4xxx, 86 = RTX 3xxx, 80 = A100
# Override at build time: docker build --build-arg CUDA_COMPUTE_CAP=86
ARG CUDA_COMPUTE_CAP=86
ENV CUDA_COMPUTE_CAP=${CUDA_COMPUTE_CAP}

# Stub nvidia-smi for build environment (real GPU only available at runtime).
# bindgen_cuda calls `nvidia-smi --query-gpu=compute_cap --format=csv`
# and expects "compute_cap\n<major>.<minor>" output.
RUN MAJOR=$(echo ${CUDA_COMPUTE_CAP} | cut -c1) && \
    MINOR=$(echo ${CUDA_COMPUTE_CAP} | cut -c2) && \
    printf '#!/bin/sh\necho "compute_cap"\necho "%s.%s"\n' "$MAJOR" "$MINOR" \
    > /usr/local/bin/nvidia-smi && chmod +x /usr/local/bin/nvidia-smi

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock* ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --features cuda && \
    rm -rf src

# Copy the actual source code
COPY . .

# Expose the port
EXPOSE 8000
ENV DB_NODES="oort-db:9042"
# Command to run with hot-reloading
CMD ["cargo", "watch", "-x", "run --features cuda", "-w", "src"]
