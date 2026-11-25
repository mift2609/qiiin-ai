#!/bin/sh
# Read-only preflight check on the Z Flip3 before any QIIIN AI deployment.
# Jalankan: sh preflight.sh  (tidak mengubah apa pun)

echo "=== identity"
uname -a
cat /etc/openwrt_release 2>/dev/null | head -6

echo "=== memory"
grep -E 'MemTotal|MemAvailable|SwapTotal' /proc/meminfo
cat /proc/swaps

echo "=== zram"
cat /sys/block/zram0/disksize 2>/dev/null || echo no-zram

echo "=== storage"
df -hT / /tmp /dev/shm
df -h /opt 2>/dev/null || echo no-opt

echo "=== cpu"
grep -c ^processor /proc/cpuinfo
cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor
for c in /sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq /sys/devices/system/cpu/cpu7/cpufreq/cpuinfo_max_freq; do printf '%s ' "$(cat "$c")"; done
echo

echo "=== binaries"
for f in /opt/qiiin-ai/bin/llama-server /opt/qiiin-ai/bin/qiiin-ai-gateway; do
	[ -x "$f" ] && echo "OK $f" || echo "MISSING $f"
done

echo "=== model"
ls -lh /opt/qiiin-ai/models/*.gguf 2>/dev/null || echo no-model

echo "=== thermal"
/usr/bin/flip3-thermal-ctl status 2>/dev/null || echo no-thermal-ctl

echo "=== done (no changes made)"
