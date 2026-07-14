<script lang="ts">
  import { fetchUsageRaw } from "../api";

  let raw = $state("");
  let loading = $state(false);

  async function load() {
    loading = true;
    raw = "";
    try {
      raw = await fetchUsageRaw();
    } catch (e) {
      raw = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="debug-raw">
  <button class="btn-ghost" onclick={load} disabled={loading}>
    {loading ? "加载中..." : "获取原始 JSON"}
  </button>
  {#if raw}
    <pre>{raw}</pre>
  {/if}
</div>

<style>
  .debug-raw {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  pre {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    font-size: 11px;
    color: var(--text-muted);
    max-height: 300px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>
