#!/bin/sh
# Build static aarch64-musl binaries for QIIIN AI Hybrid Server.
# On the laptop CROSS_CC defaults to the local Buildroot musl toolchain.
# In CI set CROSS_CC to the musl.cc cross compiler, e.g.
#   CROSS_CC=$HOME/toolchains/aarch64-linux-musl-cross/bin/aarch64-linux-musl-gcc
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
CROSS_CC=${CROSS_CC:-/home/qiiin/toolchains/aarch64--musl--stable-2024.05-1/bin/aarch64-buildroot-linux-musl-gcc}
CROSS_CXX=${CROSS_CXX:-${CROSS_CC%gcc}g++}
CROSS_AR=${CROSS_AR:-${CROSS_CC%gcc}ar}
LLAMA_SRC=${LLAMA_SRC:-$ROOT/inference/llama.cpp}
LLAMA_BLD=${LLAMA_BLD:-$ROOT/inference/build-musl}
OUT=$ROOT/artifacts

command -v "$CROSS_CC" >/dev/null 2>&1 || { echo "missing CROSS_CC: $CROSS_CC" >&2; exit 1; }
mkdir -p "$OUT/bin"

echo "== cross compiler: $CROSS_CC"
"$CROSS_CC" --version | head -1

# ---------- llama.cpp ----------
if [ ! -d "$LLAMA_SRC/.git" ]; then
  git clone --depth 1 https://github.com/ggml-org/llama.cpp.git "$LLAMA_SRC"
fi
mkdir -p "$LLAMA_BLD"
cat > "$LLAMA_BLD/toolchain.cmake" <<EOF
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)
set(CMAKE_C_COMPILER "$CROSS_CC")
set(CMAKE_CXX_COMPILER "$CROSS_CXX")
EOF

cmake -S "$LLAMA_SRC" -B "$LLAMA_BLD" \
  -DCMAKE_TOOLCHAIN_FILE="$LLAMA_BLD/toolchain.cmake" \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_NATIVE=OFF \
  -DGGML_CPU_ARM_ARCH="armv8.2-a+dotprod+fp16" \
  -DGGML_STATIC=ON \
  -DLLAMA_BUILD_COMMON=ON \
  -DLLAMA_BUILD_TOOLS=ON \
  -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_TESTS=OFF \
  -DLLAMA_BUILD_SERVER=ON \
  -DLLAMA_CURL=OFF \
  -DCMAKE_EXE_LINKER_FLAGS="-static"
cmake --build "$LLAMA_BLD" -j"$(nproc)" --target llama-server llama-cli
cp "$LLAMA_BLD/bin/llama-server" "$LLAMA_BLD/bin/llama-cli" "$OUT/bin/"

# ---------- Rust gateway ----------
rustup target add aarch64-unknown-linux-musl
export CC_aarch64_unknown_linux_musl="$CROSS_CC"
export AR_aarch64_unknown_linux_musl="$CROSS_AR"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$CROSS_CC"
cargo build --release --target aarch64-unknown-linux-musl \
  --manifest-path "$ROOT/backend/Cargo.toml"
cp "$ROOT/backend/target/aarch64-unknown-linux-musl/release/qiiin-ai-gateway" "$OUT/bin/"

mkdir -p "$OUT/www"
cp -a "$ROOT/frontend/dist/." "$OUT/www/"
cp -a "$ROOT/deploy/." "$OUT/deploy/" 2>/dev/null || true
cp -a "$ROOT/docs/." "$OUT/docs/" 2>/dev/null || true

echo "== artifacts"
find "$OUT" -type f -maxdepth 4 -printf '%p %s bytes\n' | sort
echo "== done"
