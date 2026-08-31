#!/system/bin/sh
# ============================================
#  BGFreeze-R v1.0.0  安装脚本
#  由 Magisk / KernelSU 安装环境直接执行
# ============================================

ui_print "BGFreeze-R v1.0.0"
ui_print "Rust 后台冻结 · 安装中"

case "$ARCH" in
  arm64|arm64-v8a) ;;
  *) abort "错误：不支持的架构 $ARCH（仅支持 arm64）" ;;
esac
ui_print "- 架构检查通过 ($ARCH)"

# 关键：二进制与脚本必须可执行（zip 内权限位可能不被保留）
chmod 0755 "$MODPATH/bin/bgfreeze" 2>/dev/null
chmod 0755 "$MODPATH/service.sh" 2>/dev/null
chmod 0755 "$MODPATH/customize.sh" 2>/dev/null
chmod 0755 "$MODPATH/uninstall.sh" 2>/dev/null
ui_print "- 权限设置完成"

mkdir -p /data/adb/bgfreeze
ui_print "- 数据目录就绪"

ui_print "- 安装完成，重启后自动生效"
ui_print "- WebUI：KSU 管理器内打开 / adb forward tcp:8765"