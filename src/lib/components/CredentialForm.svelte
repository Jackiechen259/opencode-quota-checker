<script lang="ts">
  import { setCredentials, clearCredentials } from "../api";

  let { onsaved }: { onsaved?: () => void } = $props();

  let ak = $state("");
  let sk = $state("");
  let saving = $state(false);
  let message = $state("");

  async function handleSave() {
    if (!ak.trim() || !sk.trim()) {
      message = "AK 和 SK 都不能为空";
      return;
    }
    saving = true;
    message = "";
    try {
      await setCredentials(ak.trim(), sk.trim());
      message = "保存成功";
      ak = "";
      sk = "";
      onsaved?.();
    } catch (e) {
      message = String(e);
    } finally {
      saving = false;
    }
  }

  async function handleClear() {
    try {
      await clearCredentials();
      message = "凭证已清除";
    } catch (e) {
      message = String(e);
    }
  }
</script>

<div class="cred-form">
  <div class="field">
    <label for="ak">Access Key ID (必须是 IAM AK，如 AKLT...)</label>
    <input id="ak" type="text" bind:value={ak} placeholder="AKLT..." autocomplete="off" />
  </div>
  <div class="field">
    <label for="sk">Secret Access Key</label>
    <input id="sk" type="password" bind:value={sk} placeholder="输入与 AK 对应的 SK" autocomplete="off" />
  </div>
  <div class="actions">
    <button class="btn-primary" onclick={handleSave} disabled={saving}>
      {saving ? "保存中..." : "保存凭证"}
    </button>
    <button class="btn-ghost" onclick={handleClear}>清除凭证</button>
  </div>
  {#if message}
    <div class="message">{message}</div>
  {/if}
</div>

<style>
  .cred-form {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }
  .message {
    margin-top: 8px;
    font-size: 12px;
    color: var(--accent);
  }
</style>
