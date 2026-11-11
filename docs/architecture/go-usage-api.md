# OpenCode Go 用量页面接口

「Go 用量」页面读取 Console 的 Go 订阅状态、本期活动明细和可选账单余额。
它与 [Go 配额接口](usage-api.md) 不同：配额接口只有三个百分比；本页需要计费周期起点、
每日费用和模型请求数，才能画出累计曲线和预测。

> 这些是 Console 网页内部接口，未收录在公开文档中。解析层 fail-closed：
> 字段缺失、非法日期、负数/小数/溢出或非 JSON 都会使该数据源报错并回退，
> 不会把无法识别的响应当成零用量。

## 请求

三个接口都使用同一个 personal API Key：

```
Authorization: Bearer <Console API Key>
Accept: application/json
User-Agent: opencode-quota-checker/<version>
```

Console 侧使用 WorkspaceMiddleware，接受 Bearer API Key；personal key 已绑定工作区，
因此**不发送** `x-opencode-org-id` / `x-org-id`。

| 接口 | 用途 | 权限 |
| --- | --- | --- |
| `GET /console/api/go/status[?fresh=true]` | Go 产品、计费周期、三个 meter | `Usage › Read`（或工作区成员权限） |
| `GET /console/api/activity/go` | 本期每日费用/请求/token、模型请求数 | `Usage › Read` |
| `GET /console/api/billing/status` | 可用余额（Extra usage 卡片，可选） | `Billing › Read` |

`fresh=true` 只用于手动刷新，跳过 Console 的短缓存。

## Go 订阅状态

`/console/api/go/status` 可能返回 `null`（没有活动订阅）。非空响应示例：

```json
{
  "product": "go",
  "cancelAtPeriodEnd": false,
  "useBalance": false,
  "access": {
    "startsAt": "2026-10-26T00:00:00.000Z",
    "meters": {
      "fiveHour": { "usedMicroCents": "120000000", "limitMicroCents": "1200000000", "resetsAt": "…" },
      "week":     { "usedMicroCents": "420000000", "limitMicroCents": "6000000000", "resetsAt": "…" },
      "month":    { "usedMicroCents": "14400000000", "limitMicroCents": "60000000000", "resetsAt": "2026-11-26T00:00:00.000Z" }
    }
  }
}
```

- `access.startsAt` 是当前计费周期起点；`month.resetsAt` 是周期终点/续订时刻。
- 每个 meter 的百分比 = `usedMicroCents / limitMicroCents * 100`（clamp 0–100）；
  `used >= limit` 视为耗尽。
- `useBalance` 是 Extra usage 开关：`true` 表示超出限额后从余额继续。
- `product`：`go` / `go-plus` / `go-teams`。

## Go 活动明细

`/console/api/activity/go` 也可能是 `null`：

```json
{
  "startsAt": "2026-10-26T00:00:00.000Z",
  "daily": [
    { "date": "2026-10-26", "totalCostMicroCents": "…", "totalTokens": "…", "totalRequests": "…" }
  ],
  "models": [
    { "provider": "opencode", "model": "deepseek-v4.1-flash", "totalRequests": "…" }
  ]
}
```

- `daily` 是本期 Go-only 的逐日数据；按费用占比把月度百分比摊到每天。
- `models` 只有请求数；模型占比按请求数计算（`<1%` / 四舍五入）。
- 后端汇总 `totalCostMicroCents`、`totalRequests`、`totalTokens` 后下发，前端不重复求和。

## 本地每日历史与活跃度格子图

Console 只返回**本期**的 `daily`（最多约 30 天），年度格子图需要的更早日期只能由应用自己记住。
每次 activity 拉取成功后，`src-tauri/src/go_history.rs` 把逐日数据合并进本地 JSON：

```
<config_dir>/opencode-quota-checker/go-history.json
{ "version": 1, "days": [ { "date": "2026-11-11", "cost_microcents": …, "requests": …, "tokens": … } ] }
```

- **合并语义**：按 `date` 覆盖写入。上游当天的行是累计值（今天会持续增长），所以后到的数据总是更新。
- **保留窗口**：只保留最近 `HISTORY_DAYS = 400` 天；超出窗口、非法日期、以及晚于“今天 + 1 天”的行被丢弃（容错 UTC 跨日）。
- **写入**：同目录临时文件 + fsync + rename，和 `config.json` 一致；写入失败只记日志，下次成功后重试。
- **读取失败**：文件缺失是正常首次运行；损坏或 `version` 未知时记录日志并按空历史启动，不会阻塞窗口。
- **IPC**：`GoOverviewDto.history`（camelCase 字段，数组元素仍是 snake_case 的 `GoActivityDay`），按日期升序，最多 400 条。

前端 `src/lib/goCalendar.ts` 把历史铺成 53 周 × 7 天（周一为每列第一天，按 UTC 日期对齐）：

- 颜色按**当天 token 数**的排名分 4 档（1 最浅、4 最深），排名而非绝对阈值，避免某一天特别大时其他格子全部趋同。
- 今天之后的格子是占位符，不计入统计；没有记录的日期是 0 档，不是错误。
- 统计：总计 Token、活跃天数、最长连续、最高单日；悬停显示当天 Token / 请求 / 费用。
- 历史起点晚于窗口起点时，页面提示“本地记录从 … 开始”，避免把空白误读为零用量。

> 局限：历史只在页面拉取 activity 时累积（打开页面、每 5 分钟自动刷新、手动刷新）。
> 若整个计费周期都没有打开过页面，该周期的日期不会被回填——Console 没有提供历史区间接口。

## 账单余额

`/console/api/billing/status` 只读取：

```json
{ "billingMode": "prepaid", "availableMicroCents": "0", "balanceMicroCents": "0" }
```

需要 `Billing › Read`。缺少权限时仅 Extra usage 卡片的余额显示 `—`，
页面其他部分和 Extra usage 开关（来自 `/go/status`）不受影响。

## 预测与曲线

以 `start = access.startsAt`、`end = month.resetsAt`、`now = Date.now()`：

- `percent = month.percent`
- `elapsed = clamp(now - start, 0, end - start)`
- `forecast = now < end && elapsed >= 1 天 && percent > 0 && !limited ? percent * (end-start) / elapsed : null`
- `perDay = percent * 86_400_000 / elapsed`
- `forecast > 100` 时 `runsOut = start + (now-start) * 100 / percent`
- 每日累计值 = `percent * 当日累计费用 / 本期总费用`；没有 `daily` 时用
  `(start, 0) → (now, percent)` 的线性近似，并在 UI 标注“估算”。

图表 y 轴通常为 0–150%，包含 100% 月度上限虚线；预测线从当前点延伸到周期末。

## 请求策略

- **按需请求**：只有打开 Go 页面时才请求；不加入后台 monitor。
- 页面挂载时请求一次，之后每 5 分钟自动刷新；手动刷新/页面重试会强制一次。
- 三个请求并发；billing 失败不影响 status/activity。
- 每次 activity 成功后立即把 `daily` 合并进本地历史（见上一节），同一份 `go://overview` 快照即可携带新格子。
- 5 分钟缓存避免页面来回切换时重复请求；401/403/400/429 不自动重试。
- Console 的 Cloudflare 可能对高频探测返回 400；应用已限制为页面活跃时的低频请求。

## 错误语义

| 条件 | 应用映射 | 页面处理 |
| --- | --- | --- |
| 401 | `AuthenticationFailed` | 摘要区提示 Key 无效，保留配额回退 |
| 403（status/activity） | `UsagePermissionDenied` | 明细区提示权限/管理员；回退配额 |
| 403（billing） | `BillingPermissionDenied` | 只有余额显示 `—` |
| 429 | `RateLimited` | 保留旧数据，稍后重试 |
| 400/5xx | `Http` | 保留旧数据并提示 HTTP 状态 |
| 非法 JSON/日期/数值 | `Parse` | 该数据源回退，不写入零值 |

## 降级

| 缺失 | 摘要 | 曲线 | Limits | Models | Extra |
| --- | --- | --- | --- | --- | --- |
| status | 用现有 Go 配额 | 用 `reset-30d` 近似 | 回退配额 | 正常 | 开关 `—` |
| activity | 正常 | 线性近似 | 正常 | 错误提示 + Console | 正常 |
| billing | 正常 | 正常 | 正常 | 正常 | 余额 `—` |
| 全部 | 配额摘要 | 线性近似 | 回退配额 | 错误提示 | `—` |

## 失效排查

```bash
curl -sS -i "https://opencode.ai/console/api/go/status" \
  -H "Authorization: Bearer $OPENCODE_API_KEY" \
  -H "Accept: application/json"

curl -sS -i "https://opencode.ai/console/api/activity/go" \
  -H "Authorization: Bearer $OPENCODE_API_KEY" \
  -H "Accept: application/json"
```

- `401` → 重新创建 Key。
- `403` → 检查 Key 的 **Usage › Read**（billing 还需 **Billing › Read**）。
- `200` 但应用报“无法识别” → 契约可能变化，对照本文更新
  `crates/opencode-core/src/go_usage.rs` / `billing.rs`。
- `400` 且响应是 Cloudflare HTML → 短时间请求过多；等待后重试，不要加快轮询。
