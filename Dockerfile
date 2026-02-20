# Combined Builder Image
FROM ubuntu:24.04 AS builder

# 1. Install system dependencies
# We install clang and lld as Barretenberg is heavily optimized for Clang
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    cmake \
    ninja-build \
    clang \
    lld \
    pkg-config \
    libssl-dev \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# 2. Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# 3. Install Go 1.25
RUN curl -L https://go.dev/dl/go1.25.0.linux-amd64.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

# 4. Install Noir/Nargo
RUN curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
ENV PATH="/root/.nargo/bin:${PATH}"
RUN /root/.nargo/bin/noirup -v 1.0.0-beta.19

# 5. Clone Barretenberg C++ source
RUN git clone --depth 1 --filter=blob:none --sparse https://github.com/AztecProtocol/aztec-packages.git /aztec-packages \
    && cd /aztec-packages \
    && git sparse-checkout set barretenberg

# 6. Build Barretenberg C++ statically
# We use Clang and disable the specific warning that fails in GCC
WORKDIR /aztec-packages/barretenberg/cpp
RUN mkdir build && cd build \
    && cmake .. -G Ninja \
       -DCMAKE_C_COMPILER=clang \
       -DCMAKE_CXX_COMPILER=clang++ \
       -DCMAKE_BUILD_TYPE=Release \
       -DBARRETENBERG_STATIC=ON \
       -DARCH=native \
       -DMOBILE=ON \
       -DCMAKE_CXX_FLAGS="-Wno-error=missing-field-initializers" \
    && ninja barretenberg

# 7. Build Rust Bridge
WORKDIR /app
COPY libnoir_ffi ./libnoir_ffi
ENV BB_LIB_DIR=/aztec-packages/barretenberg/cpp/build/lib
RUN cd libnoir_ffi && cargo build --release --features native-backend

# 8. Merge libraries into a single static archive
# Using /bin/bash to ensure the heredoc for ar -M works correctly
RUN /bin/bash -c "ar -M <<EOT \n\
CREATE libbarretenberg_ffi.a \n\
ADDLIB /app/libnoir_ffi/target/release/libbarretenberg_ffi.a \n\
ADDLIB /aztec-packages/barretenberg/cpp/build/lib/libbarretenberg.a \n\
SAVE \n\
END \n\
EOT"

# 9. Run Tests inside Docker
COPY go.mod ./
COPY bindings.go bindings_test.go ./
COPY testdata ./testdata

# Compile circuit
RUN cd testdata/circuit && nargo compile

# Run Go tests using the library we just built
RUN CGO_LDFLAGS="-L/app -lbarretenberg_ffi -lm -ldl -lpthread" go test -v .

# Final stage: Export artifacts
FROM alpine:latest
WORKDIR /dist
COPY --from=builder /app/libbarretenberg_ffi.a .
COPY --from=builder /app/libnoir_ffi/barretenberg_ffi.h .

# Default command to copy artifacts to a mounted volume
CMD ["cp", "libbarretenberg_ffi.a", "barretenberg_ffi.h", "/out/"]
