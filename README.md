# VOLC Status

> 火山方舟 Agent Plan AFP 配额监控控制台

一个基于 Tauri 2 + Svelte 5 的桌面应用,用于实时监控火山方舟(Volcano Ark)Agent Plan 的 AFP 配额使用情况,支持后台轮询、阈值告警与桌面通知。

![license](https://img.shields.io/badge/license-Apache--2.0-blue)
![tauri](https://img.shields.io/badge/Tauri-2-orange)
![svelte](https://img.shields.io/badge/Svelte-5-ff3e00)

## 功能特性

- **安全凭据管理** - Access Key / Secret Key 通过系统钥匙串(keyring)加密存储,不以明文落盘。
- **配额卡片** - 直观展示 5 小时 / 近一周 / 近一月三个窗口的配额、已用量、剩余量、使用百分比与重置倒计时。
- **悬浮窗** - 可置顶的小型悬浮组件,在桌面随时查看关键配额。
- **后台监控与告警** - 可配置轮询间隔与各窗口告警阈值,达到阈值时通过桌面通知提醒。
- **原始响应调试** - 透传 `GetAFPUsage` 的原始 JSON,方便排查接口问题。
- **火山引擎签名** - 内置 HMAC-SHA256 V4 签名实现,直接调用方舟 OpenAPI。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2 |
| 前端 | Svelte 5 + TypeScript + Vite |
| 后端 | Rust(edition 2021) |
| 凭据存储 | `keyring`(系统原生钥匙串) |
| HTTP | `reqwest`(rustls-tls) |
| 签名 | `sha2` / `hmac`(火山引擎签名 V4) |

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/tools/install) ≥ 1.77.2
- Tauri 2 的系统依赖,参见 [Tauri 官方文档](https://v2.tauri.app/start/prerequisites/)

### 安装与开发

```bash
# 安装前端依赖
npm install

# 以开发模式启动(同时拉起 Tauri + Vite)
npm run tauri:dev
```

### 构建生产包

```bash
npm run tauri:build
```

产物输出在 `src-tauri/target/release/bundle/`。

## 使用说明

1. 首次启动后,在设置面板中填入火山方舟的 Access Key 与 Secret Key(可通过控制台「API 访问密钥」获取)。
2. 凭据将保存到系统钥匙串,下次启动自动加载。
3. 主界面查看各窗口配额卡片;可开启悬浮窗常驻桌面。
4. 在设置中配置轮询间隔与告警阈值,开启后台监控后,达到阈值将收到桌面通知。

> 注意:凭据仅存储于本机系统钥匙串,不会上传至任何服务器。

## 项目结构

```
voac-status/
├── src/                          # Svelte 前端
│   ├── App.svelte
│   ├── lib/
│   │   ├── api.ts                # Tauri 命令调用封装
│   │   ├── floatWindow.ts         # 悬浮窗逻辑
│   │   ├── types.ts
│   │   └── components/            # QuotaCard / FloatWidget / SettingsPanel ...
├── src-tauri/                     # Rust 后端
│   └── src/
│       ├── main.rs / lib.rs       # 应用入口
│       ├── client.rs              # 方舟 OpenAPI 调用
│       ├── signing.rs             # 火山引擎签名实现
│       ├── credential.rs          # 凭据存取(keyring)
│       ├── monitor.rs             # 后台监控 / 阈值告警
│       ├── commands.rs            # 暴露给前端的 Tauri 命令
│       └── models.rs              # 响应模型与数据转换
└── package.json
```

### 暴露的 Tauri 命令

| 命令 | 说明 |
|---|---|
| `set_credentials` / `has_credentials` / `clear_credentials` | 凭据的增删查 |
| `fetch_usage` | 拉取并解析后的配额报告 |
| `fetch_usage_raw` | 拉取原始 JSON 响应(调试用) |
| `start_monitor` / `stop_monitor` / `get_monitor_status` | 后台监控控制 |

## 许可证

本项目基于 [Apache License 2.0](./LICENSE) 开源。

Copyright 2026 Jackie Chen
