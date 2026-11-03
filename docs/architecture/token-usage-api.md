# OpenCode Console Token 用量接口

应用通过 OpenCode Console 的用量汇总接口读取**当前工作区的 Token 用量**，用于仪表盘上的
“Token 用量”卡片。它与 [Go 用量接口](usage-api.md)是两个独立的数据源：Go 接口只给配额百分比，
Token 接口给输入/输出/缓存 token、请求数和折算费用。

> 该接口属于 Console 网页内部使用的接口，未收录在公开文档中。解析层 fail-closed：
> 字段缺失、类型变化或非 JSON 一律报错，绝不把无法识别的响应当成零用量。

## 请求

```
GET https://opencode.ai/console/api/usage/summary?range=30d&includeLegacyKeys=true
Authorization: Bearer <Console API Key>
Accept: application/json
User-Agent: opencode-quota-checker/<version>
```

- **API Key**：与 Go 配额使用同一枚 `oc_sk_...` personal API key；它已绑定创建时选择的工作区，
  因此**不发送** `x-opencode-org-id`（Go 接口的头）或 `x-org-id`。
- **权限**：Key 需要 **Usage › Read**。缺少时返回 403；应用提示“缺少 Usage › Read 权限”，
  并提供打开 Console 的按钮。Token 权限失败不影响 Go 配额卡片。
- **range**：应用当前固定 `30d`（UTC 自然日滚动，含今天）。Console 网页还支持
  `24h` / `7d` / `6m` / `all`，本应用的接口预留了 `range` 字段但暂不提供切换。
- **includeLegacyKeys**：与 Console 网页默认一致，包含旧 Key 的用量，避免低估工作区消耗。

## 成功响应

```json
{
  "totalRequests": "123",
  "totalInputTokens": "1000",
  "totalOutputTokens": "2000",
  "totalCacheReadTokens": "3000",
  "totalCacheWrite5mTokens": "40",
  "totalCacheWrite1hTokens": "60",
  "totalCostMicroCents": "30000000",
  "services": []
}
```

所有数值都以**十进制字符串**返回（Console 内部是 `BigInt`）；解析器同时接受 JSON 整数。

| 字段 | 含义 |
| --- | --- |
| `totalRequests` | 窗口内请求数 |
| `totalInputTokens` | 未命中缓存的输入 token |
| `totalOutputTokens` | 输出 token |
| `totalCacheReadTokens` | 缓存读取 token |
| `totalCacheWrite5mTokens` / `totalCacheWrite1hTokens` | 5 分钟 / 1 小时缓存写入 token |
| `totalCostMicroCents` | 按模型牌价折算的费用，单位 microcents |
| `services` | Web 搜索等附加服务；应用当前忽略 |

## 归一化

| 应用字段 | 来源 |
| --- | --- |
| `requests` | `totalRequests` |
| `input_tokens` | `totalInputTokens` |
| `output_tokens` | `totalOutputTokens` |
| `cache_read_tokens` | `totalCacheReadTokens` |
| `cache_write_tokens` | `totalCacheWrite5mTokens + totalCacheWrite1hTokens` |
| `total_tokens` | 上述四个 token 桶之和（与 Console 的 `totalTokens` 一致） |
| `cost_microcents` | `totalCostMicroCents` |
| `fetched_at` | 本地解析时刻（毫秒） |

**费用换算**：`1 USD = 100,000,000 microcents`，即 `dollars = cost_microcents / 1e8`。
前端只做显示换算，不修改原始整数。

## 统计口径

- **工作区级**：返回 API Key 所属工作区的汇总，不是单机、单 Key 或单会话。
- **全部模型**：包含 Go 模型、Console 模型、自带 Provider 以及 Web 搜索费用。
  因此它不一定与 Go 配额窗口的百分比一一对应；卡片副标题会标注“全部模型”。
- **费用是折算值**：按模型牌价估算，不是实际扣费；Go 订阅内用量不产生额外账单。

## 错误语义

| 状态码 / 情况 | 应用映射 | 用户可见文案 |
| --- | --- | --- |
| `401` | `AuthenticationFailed` | API Key 无效或已被撤销 |
| `403` | `UsagePermissionDenied` | 缺少 Usage › Read 权限，或当前账号无权查看工作区用量 |
| `429` | `RateLimited` | 请求过于频繁，请稍后重试 |
| 其它 4xx/5xx | `Http` | 服务请求失败（HTTP nnn），响应体截断到 500 字符 |
| 非 JSON / 字段缺失 / 负数 / 小数 / 溢出 | `Parse` | 无法识别的 Token 用量数据 |

## 刷新与节流

- 后台监控：Token 请求最多**每 5 分钟**一次；Go 配额仍按用户配置的 30–3600 秒轮询。
- 手动刷新、保存/测试凭据：强制请求一次（若已有请求在飞则跳过）。
- Token 请求在独立任务中执行，不阻塞、不影响配额请求；失败只写 Token 状态。
- 保存新 Key 或清除凭据会递增 generation，旧请求的响应会被丢弃，避免覆盖新 Key 的数据。

## 失效排查

```bash
curl -sS -i "https://opencode.ai/console/api/usage/summary?range=30d&includeLegacyKeys=true" \
  -H "Authorization: Bearer $OPENCODE_API_KEY" \
  -H "Accept: application/json" \
  -H "User-Agent: opencode-quota-checker/0.0.0"
```

1. `401` → 重新创建 Key；`403` → 在 Console 的 Key 权限中勾选 **Usage › Read**。
2. `200` 但应用报“无法识别的 Token 用量数据” → 契约可能变化，用上面的命令对照字段表更新
   `crates/opencode-core/src/token_usage.rs`。
3. 连续快速请求偶尔会被 Cloudflare 以 `400` 拦截；应用已做 5 分钟节流，不要把轮询调得更快。
