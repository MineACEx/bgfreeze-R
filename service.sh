#!/system/bin/sh
# BGFreeze-R 自启脚本（boot 阶段，Root 环境）
MODDIR=${0%/*}
BIN="$MODDIR/bin/bgfreeze"

[ -x "$BIN" ] || exit 0
# 防多实例
pidof bgfreeze >/dev/null 2>&1 && exit 0

mkdir -p /data/adb/bgfreeze/logs
nohup "$BIN" --config /data/adb/bgfreeze/config.json \
  --webroot "$MODDIR/webroot" --port 8765 \
  >> /data/adb/bgfreeze/logs/daemon_stdout.log 2>&1 &