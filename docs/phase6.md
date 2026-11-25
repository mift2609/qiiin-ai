# Phase 6 — Internet & knowledge base

Endpoint baru di gateway (semuanya butuh bearer token):

- `POST /v1/search` — `{query, limit?}`. Provider `brave` atau `tavily` dari
  `QIIIN_SEARCH_PROVIDER` + `QIIIN_SEARCH_API_KEY`. Tanpa key → 503, tidak
  mengklaim hasil. Cache TTL 10 menit di memory.
- `POST /v1/web/read` — `{url}`. HTTP/HTTPS saja; blokir localhost, private,
  link-local, metadata, CGNAT, multicast; cek ulang IP setelah DNS; redirect
  dibatasi 4 hop; max 1 MB; tidak menjalankan JS.
- `POST /v1/knowledge/add` — `{title, url, content}`.
- `POST /v1/knowledge/search` — `{query, limit?}`.

## Knowledge base

Toolchain musl yang dipakai tidak menyediakan fitur `fts5` pada `rusqlite`
bundled, jadi gateway memakai **fallback pencarian teks** sesuai dokumen:
JSON di `QIIIN_KB_PATH` (default `/opt/qiiin-ai/knowledge.db`), dedupe per
konten, maksimal 500 dokumen. Perilaku retrieval tetap sama untuk RAG.

## Konfigurasi

Tambahkan ke `/etc/qiiin-ai/gateway.env` (mode 0600) hanya jika provider dipakai:

```
QIIIN_SEARCH_PROVIDER=brave
QIIIN_SEARCH_API_KEY=...
QIIIN_KB_PATH=/opt/qiiin-ai/knowledge.db
```

Jangan hardcode API key di repo.

## Verifikasi device

- search tanpa key → 503
- web reader `http://127.0.0.1/` → 400 (SSRF diblokir)
- web reader `https://example.com` → 200, teks ter-extract
- knowledge add → `inserted: 1`; search → `count: 1`
