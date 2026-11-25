# LuCI app — QIIIN AI Gateway

App LuCI di `luci/` menyediakan menu **Services → QIIIN AI** untuk:

- melihat status model, inference, gateway, bind, token (masked), search, RAM, suhu;
- **mengatur inference** (context/thread/batch/slot), provider search + API key, dan bearer token — semuanya lewat helper allowlisted;
- start/stop/restart service, enable/disable boot;
- **Buka akses LAN** → mengikat gateway ke `192.168.44.1:8080` (USB) dan `192.168.45.1:8080` (WiFi HP) lewat helper allowlisted;
- **Buka Web AI** → `http://192.168.44.1:8080/` (USB/laptop).
- **Buka Web AI (HP/AP)** → `http://192.168.45.1:8080/` (WiFi HP/iPhone).

`llama-server` tetap `127.0.0.1:8081` dan tidak pernah diekspos. Firewall tidak diubah;
akses LAN mengandalkan zone `lan` yang sudah mencakup `br-usb`.

## File

```
luci/root/usr/share/luci/menu.d/luci-app-qiiinai.json
luci/root/usr/share/rpcd/acl.d/luci-app-qiiinai.json
luci/htdocs/luci-static/resources/view/qiiinai/gateway.js
deploy/scripts/qiiin-ai-ctl
```

Semua aksi UI dipanggil lewat `/usr/bin/qiiin-ai-ctl` dengan subcommand tetap.
ACL rpcd hanya mengizinkan `exec` pada helper itu — tidak ada arbitrary shell.

## Pasang (butuh persetujuan device)

```sh
cp luci/root/usr/share/luci/menu.d/luci-app-qiiinai.json /usr/share/luci/menu.d/
cp luci/root/usr/share/rpcd/acl.d/luci-app-qiiinai.json /usr/share/rpcd/acl.d/
cp luci/htdocs/luci-static/resources/view/qiiinai/gateway.js \
   /www/luci-static/resources/view/qiiinai/gateway.js
cp deploy/scripts/qiiin-ai-ctl /usr/bin/qiiin-ai-ctl
chmod 0755 /usr/bin/qiiin-ai-ctl
/etc/init.d/rpcd restart
```

Gateway juga perlu binary terbaru yang menyajikan UI statis (`QIIIN_WWW_DIR`),
karena halaman Web AI dilayani dari gateway yang sama dengan API.
