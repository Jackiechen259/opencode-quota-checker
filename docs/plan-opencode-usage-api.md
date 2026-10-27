# 改造计划：从 HTML 抓取迁移到 OpenCode Go 用量 JSON API

> 状态：已实施（Phase 0 的真实 Key 契约验证仍需用一枚实际 API Key 执行，见 §10）
> 日期：2026-10-26
> 影响面：`crates/opencode-core`（数据层）、`src-tauri`（凭据 / 命令 / 状态）、`src`（凭据页 / 调试页 / 类型）、README

---

## 1. 调研结论

### 1.1 现状（v0.1.x）

配额来自登录后的 HTML 面板 `https://opencode.ai/workspace/<id>/go`，携带浏览器 `auth` Cookie，由 `parser.rs` 分两层（SSR `usagePercent` / `resetInSec` → DOM `data-slot`）解析，失败即报解析错误。

问题：无契约的 HTML 结构，Solid hydration 输出顺序会变，历史上已经踩过（见 `parser.rs` 里 `duplicate_key_without_values_does_not_shadow_real_window` 等测试）；且 `resetInSec` 是相对秒数，需要本地 `now + resetInSec` 推算，时钟漂移会污染重置时间。

### 1.2 新方式（已确认存在，实测可用）

**`GET https://opencode.ai/zen/go/v1/usage`**

```bash
curl -sS -i https://opencode.ai/zen/go/v1/usage \
  -H "Authorization: Bearer oc_sk_..." \
  -H "Accept: application/json" \
  -H "x-opencode-org-id: <workspace-id>"
```

未带令牌实测返回（2026-10-26）：

```
HTTP/1.1 401
{"type":"error","error":{"type":"AuthError","message":"Unauthorized"}}
```

成功响应（结构来自 opencode 官方 schema）：

```json
{
  "usage": {
    "rolling": { "status": "ok",           "percent": 78.0, "resetsAt": "2026-10-27T05:31:52.000Z" },
    "weekly":  { "status": "ok",           "percent": 52.0, "resetsAt": "2026-10-31T00:00:00.000Z" },
    "monthly": { "status": "rate-limited", "percent": 100.0, "resetsAt": "2026-11-01T00:00:00.000Z" }
  }
}
```

字段语义：

| 字段 | 含义 |
| --- | --- |
| `percent` | 该窗口额度**已用百分比**，区间 0–100（与现有 `usagePercent` 同义） |
| `status` | `ok` \| `rate-limited`；`rate-limited` 表示该窗口已耗尽、Go 拒绝请求 |
| `resetsAt` | 该窗口重置的**绝对时间**（ISO-8601 字符串，毫秒精度） |

错误语义：

| 状态码 | 含义 | 处理 |
| --- | --- | --- |
| `401` | 凭证无效 / 已撤销 / 已过期 | 视为认证失败 |
| `403` | 该工作区未订阅 Go，或 API Key 权限不足 | 新增独立错误类型，提示文案要包含两种可能 |
| `429` | 限流 | 沿用现有 `RateLimited` |

### 1.3 证据链

| 来源 | 内容 |
| --- | --- |
| `packages/core/src/usage-limits.ts`（opencode `dev`） | `readGo()`：`GET ${origin}/zen/go/v1/usage`，`Authorization: Bearer …`，`x-opencode-org-id: <workspaceID>`，403 视为未订阅 Go |
| `packages/schema/src/usage-limits.ts` | `GoMeter { percent: 0..100, limited: boolean, resetsAt: epoch ms }`；`Go { rolling, weekly, monthly }` |
| `packages/server/src/handlers/usage.ts` | 本地服务把同一份数据暴露为 `usage.limits`（OpenAPI operationId `experimental.usage.limits`） |
| `packages/core/src/console-access.ts` | Console 默认 base `https://opencode.ai/console`；Console 侧头是 `x-org-id`（**注意与 Go 用量接口的 `x-opencode-org-id` 不同**） |
| <https://opencode.ai/v2/docs/console/api> | API Key 权限矩阵包含 **Usage › Read** |
| <https://opencode.ai/v2/docs/console/go> | Go 额度按 5 小时 / 每周 / 每月三窗口，响应里以 `opencode.allowance` 回报 |
| 实测 | `/zen/go/v1/usage` → 401；`/console/api/policies/current-budgets` → 401 `{"_tag":"Unauthorized"}` |

### 1.4 与旧方案对比

| 维度 | 旧（HTML + Cookie） | 新（JSON + API Key） |
| --- | --- | --- |
| 契约 | 无，DOM 结构随版本变 | 结构化 JSON，字段稳定 |
| 凭证 | 浏览器 `auth` Cookie（会随登出 / 过期失效，含其它会话语义） | Console API Key，可命名、可设过期、可随时撤销 |
| 重置时间 | `now + resetInSec` 推算 | 服务端绝对时间戳 |
| 限流标志 | 无，只能靠 100% 猜测 | `status: "rate-limited"` 权威标志 |
| 解析代码 | 约 330 行多策略解析 + 实体解码 + 时长解析 | 约 60 行 serde 映射 |
| 用户操作 | 开发者工具里翻 Cookie | Console → Account → Personal API keys → 复制 |

### 1.5 风险与未确认点（必须在 Phase 0 落实）

1. **接口未进公开文档**：它是 opencode 客户端自己在用的接口，与旧 HTML 同属"非公开契约"，但结构稳定得多。缓解：解析层 fail-safe + 保留原始响应调试浮层（已有能力）。
2. **`x-opencode-org-id` 是否必需**：API Key 本身已绑定单个 workspace（文档："Each key works in one workspace, no workspace header is needed"），但 opencode 自己始终会带该头。→ Phase 0 用真实 Key 分别测"带头 / 不带头"。
3. **Key 需要哪些权限**：权限矩阵有 `Usage › Read`，但该路由挂在 `/zen/go/` 下，可能同时要求 `Models › Request`。→ Phase 0 验证；文档里建议用户至少勾选 **Usage: Read**，若 403 再补 **Models: Request**。
4. **Workspace ID 从哪来**：Console 界面里可见；如果 2 成立（头可省），这个字段就可以变成可选项。

---

## 2. 目标与非目标

**目标**

- 配额数据源切换为 `/zen/go/v1/usage`，删除 HTML 解析路径。
- 凭据从 `auth` Cookie 改为 Console API Key，仍只存系统钥匙串。
- 重置时间改用服务端 `resetsAt`，并保留/新增限流标志。
- 老用户有明确的迁移提示，不出现"看起来正常但数据是 0"的假象。

**非目标**

- 不做 OAuth 浏览器登录流程（API Key 更简单、可撤销）。
- 不接入 `current-budgets`（Console 预算）——可作为后续独立需求，见 §9。
- 不改悬浮窗布局、通知策略、更新器、托盘逻辑。

---

## 3. 设计

### 3.1 数据流

```
keyring(opencode-api-key) ──► credential_task::load_api_key()
                                      │
config.opencode_workspace_id ─────────┤
                                      ▼
                    OpenCodeClient::fetch_usage(key, workspace?)
                                      │  GET /zen/go/v1/usage
                                      ▼
                    usage::parse_usage(json, now_ms)  ──► UsageReport
                                      │
                                      ▼
                 monitor::run_once → AppState → events → React / 悬浮窗
```

### 3.2 凭据模型

- 新增钥匙串条目：`service=opencode-quota-checker`、`account=opencode-api-key`。
- 保留旧条目 `account=opencode-auth`（不主动删除），只用于"检测到旧凭据"提示与回滚。
- `OpenCodeAuthStore` 重命名/扩展为 `OpenCodeCredentialStore`：
  - `save_key / load_key / clear_key`
  - `has_legacy_cookie() -> bool`（读旧条目）
  - `clear()` 同时清理两个条目
- **迁移 UX**：启动时若只有旧 Cookie、没有 API Key → 凭据页顶部显示一次性提示"旧版 Auth Cookie 已停用，请改用 API Key"，并给出去 Console 建 Key 的按钮。
- 保存流程增加**即时校验**：保存后立刻跑一次 `usage` 请求，把结果回显在凭据页（成功 / 401 / 403 分别给不同文案），避免用户以为保存成功。

### 3.3 配置

`AppConfig` 字段**不改名**（避免 `SCHEMA_VERSION` 迁移）：

- `opencode_workspace_id`：语义从"面板路径里的 ID"变为"发给 `x-opencode-org-id` 的 workspace ID"，**变为可选**。
- `AppState::configured()` 判定改为：`has_api_key() && (workspace_id 为空 或 非空)` → 实际只需要 Key。若 Phase 0 证明该头必需，则保持"Key + Workspace 都必填"。
- `SCHEMA_VERSION` 维持 `2`；仅当最终决定改字段名时才升到 `3` 并在 `store.rs` 写迁移。

### 3.4 `opencode-core` 改动

| 文件 | 动作 |
| --- | --- |
| `src/client.rs` | 删 `fetch_dashboard`；新增 `fetch_usage(api_key, workspace_id) -> Result<GoUsageResponse, OpenCodeError>`，`bearer_auth` + `Accept: application/json` + `User-Agent: opencode-quota-checker/<ver>`（Go 文档要求客户端自报 UA）+ 可选 `x-opencode-org-id`；状态码映射见 §1.2 |
| `src/parser.rs` | **删除**（HTML 策略、`is_login_page`、`parse_duration`、`decode_html_entities` 及其测试） |
| `src/usage.rs`（新） | serde 结构体 + `parse_usage(json, now_ms) -> Result<UsageReport, OpenCodeError>`；核心映射见下 |
| `src/quota.rs` | `fetch_quota(&self, api_key, workspace_id: Option<&str>)`；`fetch_raw_dashboard` → `fetch_raw_usage`（返回原始 JSON） |
| `src/models.rs` | `WindowReport` 增加 `#[serde(default)] pub limited: bool`；其余不变 |
| `src/error.rs` | 新增 `NotSubscribed`；`AuthenticationFailed` 文案改为"API Key 无效或已撤销/过期"；`Parse` 文案改为"用量接口返回了无法识别的数据" |
| `src/credential.rs` | 见 §3.2 |
| `src/lib.rs` | 模块导出改为 `usage`，移除 `parser` |

解析映射（关键，保证 UI 不变）：

| 新 | 旧 | 说明 |
| --- | --- | --- |
| `rolling` | `rolling-5h` / `5 小时` | 键与文案保持不变 |
| `weekly` | `weekly` / `近一周` | |
| `monthly` | `monthly` / `近一月` | |
| `percent` | `used`、`percent` | `clamp(0,100)` |
| `100 - percent` | `remaining` | |
| `quota = 100` | 同 | OpenCode 只给百分比，沿用归一化 |
| `resetsAt` | `reset_time` | 直接取服务端毫秒时间戳（不再 `now + resetInSec`） |
| `(resetsAt - now)/1000` | `reset_in_secs` | 负值截断为 0 |
| `resetsAt` | `subscribe_time` | 仍作为告警去重周期，语义更准确 |
| `status == "rate-limited"` | 无 | 写入 `limited` |
| — | `plan_type` | 接口不返回套餐档位，保持空串 |

时间解析：`chrono::DateTime::parse_from_rfc3339`，失败即 `Parse` 错误（**不猜测、不填 0 冒充正常**）。

请求失败要保证凭证不外泄：错误体仍走现有 `truncate(…, 500)`，`Authorization` 头不进日志。

### 3.5 `src-tauri` 改动

| 文件 | 动作 |
| --- | --- |
| `src/credential_task.rs` | `load_cookie/save_cookie` → `load_api_key/save_api_key`；`check_credentials` 改为检查 API Key；阻塞池 + 软超时机制完全复用 |
| `src/commands/credentials.rs` | `save_credentials(workspace_id: Option<String>, api_key: String)`；保存后就地校验并返回结果；`clear_credentials` 一并清理旧 Cookie；新增 `test_connection` 命令 |
| `src/commands/quota.rs` | `get_raw_dashboard` → `get_raw_usage`（返回 pretty JSON） |
| `src/monitor.rs` | 取 key 与可选 workspace 后调用 `fetch_quota` |
| `src/state.rs` | `configured()` 语义调整；`UsageState.raw` 内容变为 JSON |
| `src/lib.rs` | 命令注册表更新（`get_raw_usage`、`test_connection`） |
| `src-tauri/tests/dto_contract.rs` | 跟随 `WindowReport.limited` 调整 |

`src-tauri/capabilities/float.json` 不变（仍在最小权限范围内）。

### 3.6 前端改动

| 文件 | 动作 |
| --- | --- |
| `src/pages/Credentials.tsx` | 表单改为「API Key」+「Workspace ID（可选）」；文案改为 Console 建 Key 路径；登录按钮改为打开 Console 的 API Key 页面；旧 Cookie 迁移提示 |
| `src/pages/Settings.tsx` | 连接卡片的 `Auth Cookie` 字段（约 362 行）、删除确认文案（518 / 535 行）改为 API Key |
| `src/pages/Debug.tsx` | `getRawDashboard` → `getRawUsage`，提示文案改为"响应可能包含工作区标识" |
| `src/services/tauri.ts` | `saveCredentials(workspaceId?, apiKey)`、`getRawUsage`、`testConnection` |
| `src/types/models.ts` | `WindowReport.limited?: boolean`（可选，避免历史测试全改） |
| 各测试 fixture | `Dashboard.test.tsx`、`Settings.test.ts`、`floatLayout.test.ts`、`FloatWindow.test.tsx` 按需补字段 |

可选的 UX 增强（建议做，成本很低）：`quotaHealth()` 在 `limited === true` 时直接判定为 critical，并在配额卡上显示「已限流」标记，不再单靠百分比推断。

### 3.7 文档

- `README.md`：§工作原理、§认证、§使用、§安全 四处重写（约 27–31 行、37–40 行、67–74 行、149–156 行）。
- 新增 `docs/architecture/usage-api.md`：接口契约、字段语义、错误码、权限要求、失效排查步骤。

---

## 4. 分阶段实施

### Phase 0 — 契约验证（0.5 天，需要一枚真实 Key）

用 curl 确认三件事，并把结论回填到 `docs/architecture/usage-api.md`：

```bash
KEY="oc_sk_..."; WS="<workspace-id>"

# 1) 基线
curl -sS -i https://opencode.ai/zen/go/v1/usage \
  -H "Authorization: Bearer $KEY" -H "Accept: application/json" \
  -H "x-opencode-org-id: $WS"

# 2) 该头是否必需
curl -sS -i https://opencode.ai/zen/go/v1/usage \
  -H "Authorization: Bearer $KEY" -H "Accept: application/json"

# 3) 只给 Usage: Read 的受限 Key 是否也能通过（决定权限建议）
```

验收：拿到一份真实 200 响应，保存为 `tests/fixtures/opencode-go/usage-ok.json`（脱敏：`resetsAt` 保留格式即可）。

### Phase 1 — 数据层（1 天）

- 新增 `usage.rs` + JSON fixtures；删除 `parser.rs` 与 HTML fixtures。
- `client.rs` 改 `fetch_usage`；`error.rs` 加 `NotSubscribed`。
- `models.rs` 加 `limited`；`quota.rs` 换签名。
- 单测覆盖：正常三窗口、`rate-limited`、百分比越界、缺一个窗口、`resetsAt` 非法、状态字符串未知。

验收：`cargo test -p opencode-core` 全绿；`cargo clippy` 无告警。

### Phase 2 — 凭据层（1 天）

- `credential.rs` 新条目 + legacy 检测；`credential_task.rs` 改名。
- `commands/credentials.rs` 保存 + 即时校验 + `test_connection`。
- wiremock 测试：200 / 401 / 403 / 429 / 头是否正确携带。

验收：保存后能立刻看到"连接成功 / Key 无效 / 未订阅 Go"。

### Phase 3 — 后端接线（0.5 天）

- `monitor.rs`、`commands/quota.rs`、`state.rs`、`lib.rs` 注册表。
- 手动刷新 / 后台轮询 / 托盘 / 悬浮窗同步验证。

### Phase 4 — 前端（1 天）

- 凭据页、设置页、调试页、类型与测试。
- 迁移提示与「已限流」展示。

### Phase 5 — 文档与发布（0.5 天）

- README / `docs/architecture/usage-api.md`。
- `cargo xtask release minor`（0.1.x → 0.2.0），发布说明里写明**破坏性变更：需要重新填写 API Key**。

总计约 **4.5 人日**。

---

## 5. 测试计划

| 层级 | 内容 |
| --- | --- |
| 单元 | `parse_usage` 全部分支；`resetsAt` 解析（Z / 带偏移 / 非法）；百分比 clamp；`limited` 映射 |
| HTTP | wiremock：200 正常、401、403、429、响应体截断；断言 `Authorization` 与 `x-opencode-org-id` 头存在/缺省时的行为 |
| 契约 | `dto_contract.rs` 随 `WindowReport.limited` 更新 |
| 前端 | `Dashboard.test.tsx`、`Settings.test.ts`、`floatLayout.test.ts`、`FloatWindow.test.tsx` |
| 端到端 | 真 Key：首次配置 → 刷新 → 托盘后台 → 悬浮窗三态 → 断开 Key 后的错误文案 |
| 回归 | 旧 Cookie 用户在升级后**必须**看到迁移提示，而不是静默失败 |

---

## 6. 兼容与回滚

- **升级路径**：旧 Cookie 条目保留不动 → 用户填新 Key → 新条目生效。想回退到 0.1.x 时旧 Cookie 仍在，能直接工作。
- **回滚成本**：数据层是独立模块替换，`git revert` 一个提交即可；配置结构未变，不需要回滚配置。
- **失败降级**：解析失败 / 未知结构一律报错，绝不用 0 用量兜底（沿用现有原则）。

---

## 7. 交付物清单

```
crates/opencode-core/src/{usage.rs 新增, client.rs, quota.rs, models.rs, error.rs, credential.rs, lib.rs 改}
crates/opencode-core/src/parser.rs                    删除
crates/opencode-core/tests/opencode_http.rs           重写
tests/fixtures/opencode-go/*.json                     新增
tests/fixtures/opencode-go/*.html                     删除
src-tauri/src/{credential_task.rs, monitor.rs, state.rs, lib.rs 改}
src-tauri/src/commands/{credentials.rs, quota.rs 改}
src-tauri/tests/dto_contract.rs                       改
src/{pages/Credentials.tsx, pages/Settings.tsx, pages/Debug.tsx 改}
src/{services/tauri.ts, types/models.ts 改}
README.md, docs/architecture/usage-api.md             改 / 新增
```

---

## 8. 需要你拍板的三个决定

1. **钥匙串条目**：新建 `opencode-api-key`（推荐，可回滚、语义清晰）还是复用 `opencode-auth`（少一个条目，但旧 Cookie 会被误读）？
2. **Workspace ID**：Phase 0 若证明头可省，是否直接把它降级为可选（推荐）并允许留空？
3. **是否顺带接入预算接口** `GET https://opencode.ai/console/api/policies/current-budgets`（返回 `limitDollars / spentDollars / resetsAt`，头是 `x-org-id`）？可以做"本月额度花费"卡片，但属于新需求，建议单独排期。

---

## 9. 后续可选增强（不在本次范围）

- 接入 `current-budgets` 展示预算用量与重置日。
- 接入本地 opencode 服务 `usage.limits`（OpenAPI `experimental.usage.limits`），用于"opencode 已登录时免配置"的场景——代价是依赖本地服务在跑。
- 用 `status: "rate-limited"` 驱动托盘图标变色。

---

## 10. 实施记录

落地时的取舍（与 §8 的三个待定项对应）：

1. **钥匙串条目**：新建 `account=opencode-api-key`；旧 `account=opencode-auth` 保留不动，`clear_credentials` 会顺手清理两者。旧 Cookie 只用于在日志里提示"需要迁移"，绝不会被发送。
2. **Workspace ID**：已降级为可选（`AppState::configured()` 只看 API Key）。Client 仅在填了 Workspace ID 时发送 `x-opencode-org-id`，因此 §1.5 第 2 条的验证结果不会阻塞使用。
3. **预算接口**：未接入，保持 §9 的单独排期。

其它实现细节：

- 新增 `crates/opencode-core/src/usage.rs`（serde + RFC 3339 解析 + 13 个单测），删除 `parser.rs` 与 5 个 HTML fixture，新增 5 个 JSON fixture。
- `WindowReport` 增加 `limited` 字段；前端 `quotaHealth(percent, limited)` 在 `limited` 时直接判为 critical。
- `save_credentials` 保存后会立即请求一次用量接口，把 401 / 403 / 网络失败当场回显；新增 `test_connection` 命令供重复测试。
- `get_raw_dashboard` 改名为 `get_raw_usage`，调试浮层展示未格式化的 JSON 原文。

尚未完成：

- **Phase 0**：用真实 API Key 跑一遍 `curl`（含"不带头 / 仅 Usage: Read 权限"两种情形），把结论补进 `docs/architecture/usage-api.md`。
- 版本号仍是 `0.2.1`：发布前执行 `cargo xtask release minor`（破坏性变更，需在发布说明里写明"必须重新填写 API Key"）。
