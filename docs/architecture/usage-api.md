# OpenCode Go 用量接口契约

应用只依赖一个数据源：OpenCode Go 的用量接口。本文记录它的请求、响应、错误语义，以及失效时怎么排查。

## 请求

```
GET https://opencode.ai/zen/go/v1/usage
Authorization: Bearer <Console API Key>
Accept: application/json
User-Agent: opencode-quota-checker/<version>
x-opencode-org-id: <workspace id>        # 可选
```

- **API Key**：在 <https://opencode.ai/console> 的 **Account › Personal API keys** 创建，建议勾选 **Usage › Read** 权限。Key 形如 `oc_sk_...`，只在创建时完整显示一次。
- **`x-opencode-org-id`**：个人 API Key 已绑定创建它的工作区，因此该头是可选覆盖项；应用仅在用户填写了 Workspace ID 时发送。
- **User-Agent**：OpenCode Go 的使用条款要求客户端自报身份，不要伪装成通用 HTTP 库。

## 成功响应

```json
{
  "usage": {
    "rolling": { "status": "ok",           "percent": 78.0,  "resetsAt": "2026-10-27T05:31:52.000Z" },
    "weekly":  { "status": "ok",           "percent": 52.0,  "resetsAt": "2026-10-31T00:00:00.000Z" },
    "monthly": { "status": "rate-limited", "percent": 100.0, "resetsAt": "2026-11-01T00:00:00.000Z" }
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `usage.rolling` | 5 小时滚动窗口 |
| `usage.weekly` | 每周窗口 |
| `usage.monthly` | 每月窗口 |
| `percent` | 该窗口额度**已用百分比**，0–100 |
| `status` | `ok` 或 `rate-limited`；后者表示该窗口额度耗尽、Go 会拒绝请求 |
| `resetsAt` | 该窗口重置的绝对时刻，RFC 3339 字符串（毫秒精度） |

三个窗口按 Go 的额度规则推导：5 小时 = 月度额度的 20%，每周 = 50%，每月 = 100%。

## 错误语义

| 状态码 | 响应体示例 | 应用映射 | 用户可见文案 |
| --- | --- | --- | --- |
| `401` | `{"type":"error","error":{"type":"AuthError","message":"Unauthorized"}}` | `AuthenticationFailed` | API Key 无效或已被撤销，请在 OpenCode Console 重新创建 |
| `403` | — | `NotSubscribed` | 该工作区未订阅 OpenCode Go，或 API Key 缺少 Usage 权限 |
| `429` | — | `RateLimited` | 请求过于频繁，请稍后重试 |
| 其它 | — | `Http { status, body }` | 服务请求失败（HTTP nnn），响应体截断到 500 字符 |

## 归一化到 100 分制

OpenCode 只给百分比，所以共享模型沿用 100 分制：

| 模型字段 | 来源 |
| --- | --- |
| `used` / `percent` | `percent`，截断到 0–100 |
| `remaining` | `100 - percent` |
| `quota` | 恒为 `100` |
| `reset_time` / `subscribe_time` | `resetsAt` 的毫秒时间戳（`subscribe_time` 同时作为告警去重周期） |
| `reset_in_secs` | `max(0, (resetsAt - now) / 1000)` |
| `limited` | `status == "rate-limited"` |

解析失败（非 JSON、字段缺失、`resetsAt` 不是合法 RFC 3339）一律返回 `Parse` 错误，**不会**回落成零用量。

## 失效排查

1. 用 curl 复现，确认是凭证问题还是契约变化：

   ```bash
   curl -sS -i https://opencode.ai/zen/go/v1/usage \
     -H "Authorization: Bearer $OPENCODE_API_KEY" \
     -H "Accept: application/json" \
     -H "x-opencode-org-id: $WORKSPACE_ID"
   ```

2. `401` → 去 Console 重新生成 Key；`403` → 检查工作区是否订阅 Go、Key 是否勾选了 **Usage › Read**。
3. 返回 `200` 但应用报「无法识别的数据」→ 契约变了，用应用内「原始响应」浮层对照本文的字段表更新 `crates/opencode-core/src/usage.rs`。

## 上游参考

该接口未收录在公开文档中，字段语义来自 OpenCode 自身实现：

- `packages/core/src/usage-limits.ts`（`readGo`）— 请求构造与 403 语义
- `packages/schema/src/usage-limits.ts` — `GoMeter` / `Go` 结构
- `packages/core/src/console-access.ts` — Console base URL 与 `x-org-id`（Console 侧头名，与用量接口的 `x-opencode-org-id` 不同）

仓库：<https://github.com/anomalyco/opencode>
