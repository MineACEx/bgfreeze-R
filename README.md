# BGFreeze-R

Rust 重构内核的 **后台冻结（进程压制）** Magisk/KernelSU 模块。

循环检测并冻结微信 / QQ / 抖音等应用的后台冗余进程，**只保留消息推送进程**，前台或宽限期内自动解冻 —— 极低内存占用、内置可视化 WebUI 与详细日志。

## 特性

- **Rust 全覆盖内核**：单二进制约 400KB，常驻内存 ~2MB，单轮循环 ~20ms
- **三重冻结机制**：SIGSTOP 挂起 + CPUSET 限核 + Renice 降权
- **消息推送保留**：微信 `:push`、QQ `:MSF`（可自定义保留进程）
- **前台保护**：应用可见（oom_adj ≤ 200）自动解冻
- **解冻宽限期**：切后台后保留数秒不冻结，短时间切回秒开（默认 120s，可调 0–300s）
- **WebUI**：主页 / 压制 / 控制 / 设置 / 日志，iPhone 毛玻璃黑白设计
  - 绿卡运行中 + 对勾；红卡未运行 + 叉（点击 3D 倾斜进入控制中心）
  - 控制中心：总开关 / 检查更新 / 下载安装更新 / 重启系统 / 卸载模块
  - 更新公告支持 Markdown 渲染
- **详细日志**：文件 + WebUI 实时查看（最近 600 条）

## 安装

1. 下载 [最新 Release](https://github.com/USER/bgfreeze-R/releases/latest) 的 zip
2. 在 KernelSU / Magisk / APatch 管理器中安装，重启生效

```sh
# 或命令行安装（root）
su -c "/data/adb/ksud module install /sdcard/Download/bgfreeze-R-v1.0.0.zip"
```

## 卸载

```sh
su -c "/data/adb/ksud module uninstall bgfreeze-R"
```

配置与日志保留在 `/data/adb/bgfreeze`，彻底清除：`rm -rf /data/adb/bgfreeze`。

## WebUI 访问

- **KSU 管理器**：模块页内直接打开
- **ADB 代理**：`adb forward tcp:8765 tcp:8765` 后浏览器访问 `http://127.0.0.1:8765`

## 更新机制

- 支持 KSU/Magisk 标准 `updateJson` **手动检查更新更快**
- WebUI「控制中心 → 检查更新」可查看 Markdown 更新公告并一键下载安装
- 更新链路：GitHub Release → `update.json` → 设备端 curl/wget 下载 → `ksud module install`

## 从源码构建（Windows → Android）

```sh
rustup override set stable-x86_64-pc-windows-gnu
cargo build --release --target aarch64-linux-android
```

- Host 链接器：`rust-lld`（无需 MSVC）
- Android 链接器：NDK `aarch64-linux-android26-clang.cmd`
- 打包注意：zip 条目须以 `/` 分隔，并为 `bin/*`、`*.sh` 设置 Unix 可执行位

## 协议

[Apache-2.0](./LICENSE)

## 致谢

思路参考社区酷安 CZero 等后台压制方案；日志排查参考 Magisk/KernelSU 生态。