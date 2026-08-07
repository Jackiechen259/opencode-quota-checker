# VOLC Status

火山方舟 Agent Plan AFP 配额的纯 Rust 原生桌面监控工具。

[![CI](https://github.com/Jackiechen259/volc_status/actions/workflows/ci.yml/badge.svg)](https://github.com/Jackiechen259/volc_status/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

## 功能

- 展示 5 小时、近一周、近一月配额、用量、剩余量与重置时间
- 原生系统托盘，关闭主窗口后可继续后台监控
- Full、Compact、Docked 三种置顶悬浮窗
- 可配置轮询间隔和每个时间窗口的告警阈值
- 原生桌面通知及同一告警周期去重
- 原始 API 响应调试浮层和一键复制
- AK/SK 仅保存到系统钥匙串，配置文件不包含敏感字段

## 技术栈

| 层 | 实现 |
|---|---|
| 桌面与 UI | Iced 0.14 daemon |
| 托盘 | `tray-icon` |
| HTTP | `reqwest` + rustls |
| 凭据 | `keyring` 系统原生后端 |
| 通知 | Windows WinRT / macOS Notification Center / Linux `notify-send` |
| 异步运行时 | Tokio |
| 打包 | `cargo-packager` |

项目运行和构建不需要 Node.js 或浏览器运行时。

## 开发

需要最新 stable Rust。Windows 和 macOS 使用系统原生工具链；Ubuntu
22.04+ 还需要：

```bash
sudo apt-get install build-essential pkg-config libgtk-3-dev \
  libayatana-appindicator3-dev libxdo-dev libsecret-1-dev libnotify-bin
```

运行应用与质量检查：

```bash
cargo run -p volc-desktop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Linux 桌面需支持 AppIndicator；部分 GNOME 环境需要启用对应扩展。X11
和 Wayland 的托盘、通知与无边框窗口行为应在发版前分别烟测。

## 使用

1. 启动后在凭据页面输入火山引擎 Access Key 和 Secret Key。
2. 点击保存；凭据写入系统钥匙串。
3. 刷新配额，按需打开后台监控、告警和悬浮窗。
4. 调试时可打开原始响应浮层；其中可能包含服务端元数据，分享前请检查。

常规设置写入系统标准配置目录下的
`volc-status/config.json`。写入采用同目录临时文件与原子替换；AK/SK
不会写入该文件。

## 目录

```text
.
├── assets/icons/             # 安装包图标
├── crates/
│   ├── volc-core/            # API、签名、模型、凭据和告警规则
│   └── volc-desktop/         # Iced 应用、视图、窗口、托盘和配置
├── docs/                     # 架构、构建、发布及迁移记录
├── spikes/iced-tray-daemon/  # Phase 1 生命周期验证原型
└── xtask/                    # Rust 版本发布工具
```

`volc-core` 不依赖桌面 UI，可独立测试。应用状态只在 Iced update
路径中修改；主窗口与悬浮窗共享同一份配额状态。

## 打包与发布

本机打包前安装：

```bash
cargo install cargo-packager --locked
cargo packager --release --config crates/volc-desktop/packager.json
```

发布版本由根 `Cargo.toml` 的 `[workspace.package].version` 管理：

```bash
cargo xtask release patch
cargo xtask release minor
cargo xtask release 1.0.0-rc.1
cargo xtask release 1.0.0 --push
```

`v*` tag 触发 Windows x64、Linux x64、macOS Intel 和 macOS Apple
Silicon 打包，并发布 SHA-256 校验和。详细步骤见
[发布说明](docs/release.md)，构建环境见[构建说明](docs/building.md)。

## 安全

- 不在日志、配置、测试 fixture 或错误信息中输出 AK/SK。
- HTTP 请求使用 rustls TLS，并设置有限超时及响应体错误截断。
- 发布包当前未配置平台代码签名；正式分发前应配置 Windows 与 macOS
  签名凭据。

## 许可证

本项目基于 [Apache License 2.0](LICENSE) 开源。第三方依赖说明见
[license notices](docs/license-notices.md)。
