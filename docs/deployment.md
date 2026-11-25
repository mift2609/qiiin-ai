# Deployment config (belum disalin ke device)

File di `deploy/` disiapkan di laptop. **Belum ada perubahan pada Z Flip3.**
Persetujuan eksplisit masih diperlukan sebelum `scp`/`cp` ke device.

## Isi

```
deploy/config/qiiin-ai.conf        # ctx 4096, threads 6, parallel 1, bind localhost
deploy/config/gateway.env.example  # token placeholder, mode 0600 saat dipakai
deploy/init.d/qiiin-llama          # procd service untuk llama-server
deploy/init.d/qiiin-gateway        # procd service untuk gateway
deploy/scripts/preflight.sh        # cek read-only sebelum deploy
```

## Alasan nilai RAM/CPU

- `-c 4096`: KV cache ~288 MB; context default model 40960 butuh ~2,8 GB.
- `-t 6` + `-tb 6`: 6 core inference, 2 core sisa untuk modem/WiFi/cover.
- `-np 1`: satu slot inference antrean, mencegah dua request berebut RAM.
- Swap/zram tidak diaktifkan; zram di device masih `disksize=0`.

## Langkah setelah disetujui

```sh
scp -r deploy root@192.168.44.1:/tmp/qiiin-deploy
ssh root@192.168.44.1 'sh /tmp/qiiin-deploy/scripts/preflight.sh'
# baru salin ke /etc/qiiin-ai, /etc/init.d, /opt/qiiin-ai/bin
```

Jangan taruh model di `/tmp` (tmpfs 3,5 GB).
