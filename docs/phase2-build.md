# Phase 2 — build di laptop / GitHub Actions

## Lokal

```sh
cd ~/repos/qiiin-ai
./scripts/build-musl.sh
```

Script memakai cross compiler Buildroot musl di
`~/toolchains/aarch64--musl--stable-2024.05-1` dan menghasilkan:

```
artifacts/bin/llama-server
artifacts/bin/llama-cli
artifacts/bin/qiiin-ai-gateway
artifacts/www/            # UI statis
artifacts/deploy/         # contoh config/init
artifacts/docs/           # dokumentasi
```

Semua binary **statik musl aarch64**. Uji di laptop sebelum kirim:

```sh
qemu-aarch64-static artifacts/bin/llama-server --version
file artifacts/bin/qiiin-ai-gateway
```

## GitHub Actions

Workflow `.github/workflows/build.yml` berjalan pada `push` ke `main` atau
`workflow_dispatch`. Runner `ubuntu-latest` mengunduh cross compiler musl.cc,
build yang sama persis, lalu meng-upload artifact `qiiin-ai-aarch64-musl`.

## Kirim ke Z Flip3

Setelah artifact diunduh ke laptop:

```sh
scp artifacts/bin/* root@192.168.44.1:/opt/qiiin-ai/bin/
scp -r artifacts/www root@192.168.44.1:/opt/qiiin-ai/
```

Jangan taruh model di `/tmp` (tmpfs 3,5 GB — ikut makan RAM).

## Konfigurasi gateway

Salin `backend/config.example.toml`, lalu set environment:

- `QIIIN_BIND` — default `127.0.0.1:8080`
- `QIIIN_AUTH_TOKEN` — wajib; kalau kosong `/v1/*` balas 503
- `QIIIN_LLAMA_BASE` — default `http://127.0.0.1:8081`
- `QIIIN_STATUS_FILE` — default `/var/run/qiiin-ai/system.json`

## Llama server (contoh)

```
llama-server -m /opt/qiiin-ai/models/Qwen3-4B-Q4_K_M.gguf \
  -c 4096 -t 6 --host 127.0.0.1 --port 8081
```

Context dibatasi 4096 agar KV cache tidak melebihi RAM.
