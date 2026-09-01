# Stage 1: Base image with CUDA, Node.js, Wine, and build tools
FROM nvidia/cuda:13.1.0-devel-ubuntu22.04 AS base

ENV DEBIAN_FRONTEND=noninteractive
ENV WINEDEBUG=-all

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    gnupg \
    cmake \
    build-essential \
    git \
    fakeroot \
    rpm \
    libarchive-tools \
    dpkg \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js 24 and enable pnpm via Corepack
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get install -y nodejs \
    && corepack enable \
    && rm -rf /var/lib/apt/lists/*

# Install Wine and NSIS for Windows cross-compilation
RUN dpkg --add-architecture i386 \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    wine64 \
    wine32 \
    nsis \
    && rm -rf /var/lib/apt/lists/*

# Stage 2: Build whisper.cpp CPU and GPU binaries
FROM base AS whisper-builder

ARG WHISPER_TAG=v1.8.3

RUN git clone --depth 1 --branch "$WHISPER_TAG" \
    https://github.com/ggerganov/whisper.cpp.git /whisper.cpp

# Build CPU binary
RUN cmake -S /whisper.cpp -B /whisper.cpp/build-cpu \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF \
      -DGGML_CUDA=OFF \
    && cmake --build /whisper.cpp/build-cpu --config Release \
      --target whisper-cli -j"$(nproc)"

# Build GPU (CUDA) binary
RUN cmake -S /whisper.cpp -B /whisper.cpp/build-gpu \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF \
      -DGGML_CUDA=ON \
      -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90" \
    && cmake --build /whisper.cpp/build-gpu --config Release \
      --target whisper-cli -j"$(nproc)"

# Copy built binaries to a clean location
RUN mkdir -p /whisper-cache \
    && cp /whisper.cpp/build-cpu/bin/whisper-cli /whisper-cache/whisper-cli-linux-x64 \
    && cp /whisper.cpp/build-gpu/bin/whisper-cli /whisper-cache/whisper-cli-linux-x64-gpu \
    && chmod +x /whisper-cache/whisper-cli-*

# Stage 3: Final builder image
FROM base AS builder

COPY --from=whisper-builder /whisper-cache/ /whisper-cache/

COPY scripts/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

WORKDIR /build

ENTRYPOINT ["/docker-entrypoint.sh"]
