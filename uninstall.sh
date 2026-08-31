#!/system/bin/sh
# ============================================
#  BGFreeze-R v1.0.0  卸载脚本（Root 环境执行）
# ============================================
echo "BGFreeze-R 卸载中..."

# 停止守护进程（按进程名精确定位，避免误杀本脚本）
DAEMON_PID=$(pidof bgfreeze 2>/dev/null)
if [ -n "$DAEMON_PID" ]; then
  kill -9 $DAEMON_PID 2>/dev/null
  echo "已停止守护进程"
else
  echo "守护进程未在运行"
fi

echo "配置与日志已保留：/data/adb/bgfreeze"
echo "彻底清除：rm -rf /data/adb/bgfreeze"
echo "卸载完成"