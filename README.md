# QIIIN AI Hybrid Server

Hybrid local-AI server for Samsung Galaxy Z Flip3 (SM-F711N) running OpenWrt.

- **Target:** aarch64 **musl**, static binaries, no runtime installed on device.
- **Inference:** llama.cpp `llama-server` + Qwen3-4B GGUF `Q4_K_M` (fallback Qwen3-1.7B).
- **Gateway:** Rust/Axum, OpenAI-compatible API, bearer auth, rate limit, streaming proxy.
- **UI:** static HTML/CSS/JS, dark graphite, no Node on device.
- **Build:** `scripts/build-musl.sh` locally, or GitHub Actions `.github/workflows/build.yml`.

See `docs/phase2-build.md` for the current build/deploy status.

## Layout

```
backend/    Rust Axum gateway
frontend/   static UI (dist/ is shipped as-is)
inference/  llama.cpp source + cross build
deploy/     init scripts and config examples
docs/       architecture, build, benchmark, recovery
scripts/    reproducible build scripts
```

Nothing here is installed on the device automatically.
