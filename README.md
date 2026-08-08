# OpenCode Quota Checker

OpenCode Go 配额的原生 Rust 桌面监控工具。展示 5 小时、近一周、近一月三个配额窗口的用量、剩余量与重置倒计时，支持系统托盘后台监控、置顶悬浮窗、阈值告警与原生桌面通知。

[![CI](https://github.com/Jackiechen259/opencode-quota-checker/actions/workflows/ci.yml/badge.svg)](https://github.com/Jackiechen259/opencode-quota-checker/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

## 功能

- 数据源：OpenCode Go（Workspace ID + auth Cookie）
- 主窗口仪表盘：最高负载、最近重置与窗口健康概览，以及每个配额窗口的用量卡
- 展示 5 小时、近一周、近一月三个配额窗口的已用、总额、剩余与重置时间
- 原生系统托盘：关闭主窗口后后台继续监控，托盘菜单可显示/隐藏主窗口、切换悬浮窗、退出
- Full、Compact、Docked 三种置顶悬浮窗，拖到屏幕顶部附近自动停靠为单行 Docked 状态
- 可配置轮询间隔（30–3600 秒）与每个窗口的告警阈值（0–100%）
- 原生桌面通知，同一重置周期内只提醒一次
- 自动更新：从 GitHub Releases 检查新版本、自动下载并校验 SHA-256，安装前需用户确认
- 原始响应调试浮层与一键复制
- auth Cookie 仅保存到系统钥匙串，配置文件不包含敏感字段

## 工作原理

OpenCode Go 尚无公开的配额 API，配额数据来自登录后的工作区面板（`https://opencode.ai/workspace/<id>/go`）。应用携带 `auth` Cookie 请求该面板，并分层解析页面结构：

1. **SSR 策略**：读取服务端渲染的 `usagePercent` / `resetInSec`（对应 `rollingUsage` / `weeklyUsage` / `monthlyUsage`）。
2. **DOM 策略**：回退到语义化 `data-slot` 属性（`usage-item`、`usage-label`、`usage-value`、`reset-time`）。
3. **安全失败**：遇到不支持的页面结构返回解析错误，而不是把零用量当成真实数据。

OpenCode 只报告百分比，应用将其归一化到 100 分制展示（总额恒为 100，已用即报告百分比，剩余与占比据此得出）。页面结构变化时会提示解析失败，而不是展示错误数据。

## 认证

- **Workspace ID**：保存在普通配置文件中（非敏感）。
- **auth Cookie**：登录 opencode.ai 后从浏览器开发者工具中获取。它被视为密码，仅保存到系统钥匙串，随请求发送到 opencode.ai，不会写入配置文件或日志。

## 开发

需要最新 stable Rust。Windows 和 macOS 使用系统原生工具链；Ubuntu 22.04+ 还需要：

```bash
sudo apt-get install build-essential pkg-config libgtk-3-dev \
  libayatana-appindicator3-dev libxdo-dev libsecret-1-dev libnotify-bin
```

运行应用与质量检查：

```bash
cargo run -p opencode-desktop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Linux 桌面需支持 AppIndicator；部分 GNOME 环境需要启用对应扩展。X11 和 Wayland 的托盘、通知与无边框窗口行为应在发版前分别烟测。

Windows 11 24H2 上 winit 0.30 存在跨 DPI 拖拽窗口时的 `WM_DPICHANGED` 反馈环问题，仓库以 `vendor/winit-0.30.13` 的方式向下游打了窄补丁（见根 `Cargo.toml` 的 `[patch.crates-io]`），上游修复前请勿移除。

## 使用

1. 点击「在浏览器中登录」，在系统浏览器中打开 opencode.ai 登录页并完成 GitHub / Google 登录。
2. 从地址栏复制 Workspace ID，在浏览器开发者工具中复制 opencode.ai 的 `auth` Cookie，填入表单保存。
3. 刷新配额，按需在设置中调整轮询间隔与告警阈值，并打开悬浮窗。
4. 调试时可打开原始响应浮层；分享前请检查其中可能包含的服务端信息。

常规设置写入系统标准配置目录下的 `opencode-quota-checker/config.json`（首次启动自动写入默认配置）。写入采用同目录临时文件与原子替换；auth Cookie 不会写入该文件。

主窗口关闭默认最小化到托盘（托盘不可用时改为退出）。主窗口头部支持 Tab / Shift+Tab 在操作间循环切换、Enter 激活、Esc 关闭调试浮层或设置。

## 自动更新

应用默认自动从官方 GitHub Releases 检查新版本（可在设置中关闭）：

- 启动后检查一次，之后每约 6 小时检查一次，仅接受 stable 版本，不会提示预发布版本。
- 发现新版本后默认自动下载（可在设置中关闭），更新包下载到系统缓存目录。
- 下载完成后校验 SHA-256，与发布清单不符时丢弃文件并提示，绝不会运行未校验的包。
- 安装前始终要求用户确认：Windows 启动安装程序、macOS 打开 DMG、Linux AppImage 安全替换并重启或打开 deb 包。
- 更新检查失败不影响额度监控，错误只在设置页显示，可随时手动重新检查。
- macOS Intel 与其它未发布平台不会收到任何更新包。

## 目录

```text
.
├── assets/icons/             # 安装包图标
├── crates/
│   ├── opencode-core/        # OpenCode 客户端、配额解析器、模型、凭据和告警规则
│   └── opencode-desktop/     # Iced 应用、视图、窗口、托盘和配置
├── docs/                     # 架构、构建与发布说明
├── tests/                    # 页面解析 fixtures 与 HTTP 集成测试
└── xtask/                    # Rust 版本发布工具
```

`opencode-core` 不依赖桌面 UI，可独立测试。应用状态只在 Iced update 路径中修改；主窗口与悬浮窗共享同一份配额状态。

## 构建

本机打包前安装：

```bash
cargo install cargo-packager --locked
cargo packager --release --config crates/opencode-desktop/packager.json
```

各平台的构建环境见[构建说明](docs/building.md)。

## 发布

发布版本由根 `Cargo.toml` 的 `[workspace.package].version` 管理：

```bash
cargo xtask release patch
cargo xtask release minor
cargo xtask release 1.0.0-rc.1
cargo xtask release 1.0.0 --push
```

`v*` tag 触发 Windows x64（NSIS）、Linux x64（deb / AppImage）和 macOS Apple Silicon（DMG）打包，自动生成 SHA256 校验和与 `update.json` 更新清单并发布。详细步骤见[发布说明](docs/release.md)。

## 安全

- 不在日志、配置、测试 fixture 或错误信息中输出 auth Cookie。
- HTTP 请求使用 rustls TLS，并设置有限超时及响应体错误截断。
- 发布包当前未配置平台代码签名；正式分发前应配置 Windows 与 macOS 签名凭据。

## 许可证

本项目基于 [Apache License 2.0](LICENSE) 开源。第三方依赖说明见 [license notices](docs/license-notices.md)。
