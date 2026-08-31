#!/system/bin/sh
# ============================================
#  BGFREEZE-R  安装脚本（Magisk / KernelSU）
#  支持 arm64-v8a / armeabi-v7a
# ============================================

ui_print "BGFREEZE-R v1.0.10"
ui_print "Rust 后台冻结 · 安装中"

# 架构 -> 选择二进制
case "$ARCH" in
  arm64|arm64-v8a)      BIN="arm64-v8a" ;;
  arm|armeabi|armeabi-v7a) BIN="armeabi-v7a" ;;
  *) abort "错误：不支持的架构 $ARCH（支持 arm64-v8a / armeabi-v7a）" ;;
esac
ui_print "- 架构 $ARCH → 使用 $BIN 二进制"

# 只保留当前架构二进制，其余删除
if [ "$BIN" != "arm64-v8a" ]; then rm -rf "$MODPATH/bin/arm64-v8a" 2>/dev/null; fi
if [ "$BIN" != "armeabi-v7a" ]; then rm -rf "$MODPATH/bin/armeabi-v7a" 2>/dev/null; fi
mv "$MODPATH/bin/$BIN/bgfreeze" "$MODPATH/bin/bgfreeze" 2>/dev/null
rm -rf "$MODPATH/bin/arm64-v8a" "$MODPATH/bin/armeabi-v7a" 2>/dev/null

# 可执行权限
chmod 0755 "$MODPATH/bin/bgfreeze" 2>/dev/null
chmod 0755 "$MODPATH/service.sh" 2>/dev/null
chmod 0755 "$MODPATH/uninstall.sh" 2>/dev/null
ui_print "- 权限设置完成"

mkdir -p /data/adb/bgfreeze
ui_print "- 数据目录就绪"

ui_print "- 安装完成，重启后自动生效"
ui_print "- WebUI：KSU 管理器内打开 / adb forward tcp:8765"