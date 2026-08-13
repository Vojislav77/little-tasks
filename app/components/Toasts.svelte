<script lang="ts">
  import { toasts } from "../lib/toasts";

  const list = $derived($toasts);
</script>

<div class="toast-stack">
  {#each list as t (t.id)}
    <div class="toast {t.kind}">
      {t.message}
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 999;
    align-items: center;
    pointer-events: none;
  }

  .toast {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 9px 14px;
    font-size: 13px;
    max-width: 90vw;
    animation: slide-in 0.18s ease;
  }

  .toast.success {
    border-color: var(--mint);
    background: var(--mint-soft);
    color: #2e6b4e;
  }

  .toast.error {
    border-color: var(--pink);
    background: var(--danger-soft);
    color: var(--danger);
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
