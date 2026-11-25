# Sumber model

Model dan bobot tidak disimpan di repo. Unduh di laptop, verifikasi, baru kirim ke device.

## Primary

- Repo: `bartowski/Qwen_Qwen3-4B-GGUF`
- File di repo: `Qwen_Qwen3-4B-Q4_K_M.gguf`
- Ukuran: 2.497.280.960 byte (~2,50 GB)
- SHA256: `fbe1d5edd4ce802ae3ae7c7e4ab7d09789d697fdac1fc7929f8df4ca3c41bae3`
- URL: `https://huggingface.co/bartowski/Qwen_Qwen3-4B-GGUF/resolve/main/Qwen_Qwen3-4B-Q4_K_M.gguf`
- Nama di device: `Qwen3-4B-Q4_K_M.gguf`

## Fallback

- Repo: `bartowski/Qwen_Qwen3-4B-GGUF`
- File: `Qwen3-4B-Q3_K_M.gguf` (2,08 GB) — lebih hemat RAM, kualitas turun.
- Alternatif ringan: `Qwen3-1.7B` Q4_K_M (~1,1 GB).

## Verifikasi

```sh
curl -L --retry 3 -o Qwen3-4B-Q4_K_M.gguf "<URL di atas>"
sha256sum Qwen3-4B-Q4_K_M.gguf
```

Catat hash hasil unduhan di sini sebelum `scp` ke Flip3. Jangan pakai `/tmp` device
karena tmpfs 3,5 GB ikut memakan RAM.
