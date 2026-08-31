# BGFreeze-R

**后台冻结（进程压制）模块 · Background Freeze / Process Suppression module**
Rust 重构内核，支持 **Magisk / KernelSU / APatch**。

周期检测并冻结微信 / QQ / 抖音等应用的后台冗余进程，**仅保留消息推送进程**，前台或宽限期内自动解冻 —— 极低内存占用，内置可视化 WebUI 与详细日志。

Periodically freezes redundant background processes of WeChat / QQ / Douyin, **keeping only the push processes**. Auto-unfreezes when foregrounded or within the grace period — minimal footprint, with a built-in WebUI and detailed logs.

---

## 特性 · Features

- **Rust 全覆盖内核 · Rust-only core**：单二进制约 400KB，常驻内存 ~2MB，单轮循环 ~20ms
- **三重冻结机制 · Triple-freeze**：SIGSTOP 挂起 + CPUSET 限核 + Renice 降权
- **消息推送保留 · Push preserved**：微信 `:push`、QQ `:MSF`（可自定义 · customizable）
- **前台保护 · Foreground protection**：应用可见（oom_adj ≤ 200）自动解冻
- **解冻宽限期 · Grace period**：默认 120s，可调 0–300s
- **WebUI**：主页 / 压制 / 设置 / 日志（Home / Freeze / Settings / Logs），iPhone 毛玻璃黑白设计（frosted-glass monochrome）
  - 绿卡工作中 + 对勾；红卡未运行 + 叉；3D 按压倾斜进入控制中心
  - 控制中心：总开关 / 检查更新 / 下载安装更新 / 重启系统 / 卸载模块
  - 更新公告支持 Markdown 渲染 · Markdown release notes
- **详细日志 · Detailed logs**：文件 + WebUI 实时查看（最近 600 条）

## 安装 · Install

1. 下载 [最新 Release](https://github.com/USER/bgfreeze-R/releases/latest) 的 zip
2. 在 KernelSU / Magisk / APatch 管理器中安装，重启生效（install in manager, reboot）

```sh
# 或命令行安装（root）
su -c "/data/adb/ksud module install /sdcard/Download/bgfreeze-R-v1.0.0.zip"
```

## 卸载 · Uninstall

```sh
su -c "/data/adb/ksud module uninstall bgfreeze-R"
```

配置与日志保留在 `/data/adb/bgfreeze`；彻底清除：`rm -rf /data/adb/bgfreeze`
（Config & logs kept at `/data/adb/bgfreeze`; full wipe with the command above）

## WebUI 访问 · Access

- **KSU 管理器**：模块页内直接打开（open from the module page）
- **ADB 代理**：`adb forward tcp:8765 tcp:8765` 后浏览器访问 `http://127.0.0.1:8765`

## 更新机制 · Updates

- 支持 KSU/Magisk 标准 `updateJson` 更新提示
- WebUI「控制中心 → 检查更新」可查看 Markdown 更新公告并一键下载安装
- 链路：GitHub Release → `update.json` → 设备端下载 → `ksud module install`

## 从源码构建 · Build (Windows → Android)

```sh
rustup override set stable-x86_64-pc-windows-gnu
cargo build --release --target aarch64-linux-android
```

- Host 链接器：`rust-lld`（无需 MSVC）
- Android 链接器：NDK `aarch64-linux-android26-clang.cmd`（armv7 同 NDK，见 `link-armv7.py`）
- 打包注意：zip 条目以 `/` 分隔，`bin/*`、`*.sh` 需设 Unix 可执行位

## 支持架构 · Architectures

- arm64-v8a（主 · primary）
- armeabi-v7a（ARM32）

## 协议 · License

[Apache-2.0](./LICENSE)

## 致谢 · Credits

思路参考社区 CZero 等后台压制方案（Inspired by CZero & community background-suppression tools）。