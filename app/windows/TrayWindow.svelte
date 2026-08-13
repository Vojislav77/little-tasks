<script lang="ts">
  import { onMount } from "svelte";
  import TaskListItem from "../components/TaskListItem.svelte";
  import { api, isTauri } from "../lib/api";
  import type { AppSettings, Task, TaskList } from "../lib/types";
  import { toastError, toastSuccess } from "../lib/toasts";

  let lists = $state<TaskList[]>([]);
  let tasks = $state<Task[]>([]);
  let listId = $state("");
  let query = $state("");
  let qTitle = $state("");
  let busy = $state(false);
  let pendingOnly = $state(false);

  let titleInput = $state<HTMLInputElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);

  const listTitleById = $derived(
    new Map(lists.map((l) => [l.id, l.title])),
  );

  const visibleTasks = $derived(
    pendingOnly ? tasks.filter((t) => !t.done) : tasks,
  );

  async function refresh() {
    try {
      tasks = query.trim() ? await api.searchTasks(query) : await api.listTasks();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function refreshLists() {
    try {
      lists = await api.listTaskLists();
      if (lists.length > 0 && !lists.some((l) => l.id === listId)) {
        listId = lists[0].id;
      }
    } catch (e) {
      toastError(String(e));
    }
  }

  async function quickAdd() {
    if (busy) return;
    if (!listId) {
      toastError("Create a task list first (open the editor).");
      return;
    }
    const title = qTitle.trim();
    if (!title) return;
    busy = true;
    try {
      await api.createTask(listId, title);
      qTitle = "";
      await refresh();
      toastSuccess("Task added");
      titleInput?.focus();
    } catch (e) {
      toastError(String(e));
    } finally {
      busy = false;
    }
  }

  async function toggleTask(task: Task) {
    try {
      await api.toggleTask(task.id);
      await refresh();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function removeTask(task: Task) {
    try {
      await api.deleteTask(task.id);
      await refresh();
    } catch (e) {
      toastError(String(e));
    }
  }

  function openInEditor(task: Task) {
    void api.openEditor(task.id);
  }

  onMount(() => {
    let dispose: (() => void) | undefined;
    void (async () => {
      await refreshLists();
      await refresh();
      titleInput?.focus();

      try {
        const settings = await api.getSettings();
        pendingOnly = settings.showPendingOnly;
      } catch {}

      if (isTauri()) {
        const { listen } = await import("@tauri-apps/api/event");
        await listen("tasks-changed", () => refresh());
        await listen<AppSettings>("settings-changed", (e) => {
          pendingOnly = e.payload.showPendingOnly;
        });

        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const win = getCurrentWindow();
        const handleKey = (ev: KeyboardEvent) => {
          if (ev.key === "Escape") {
            api.hideTray();
          } else if (ev.ctrlKey && !ev.shiftKey && !ev.altKey) {
            if (ev.key === "n" || ev.key === "N") {
              ev.preventDefault();
              api.newTask();
            } else if (ev.key === "f" || ev.key === "F") {
              ev.preventDefault();
              searchInput?.focus();
              searchInput?.select();
            }
          }
        };
        window.addEventListener("keydown", handleKey);

        // Focus loss hides the popover (defense in depth with the Rust-side handler).
        const unlistenFocus = await win.onFocusChanged(({ payload }) => {
          if (!payload) api.hideTray();
        });

        dispose = () => {
          window.removeEventListener("keydown", handleKey);
          unlistenFocus();
        };
      }
    })();
    return () => dispose?.();
  });
</script>

<main class="popover">
  <header data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <span class="logo">📌</span>
      <span>Little Tasks</span>
    </div>
    <div class="header-actions">
      <button class="icon" title="New task (Ctrl+N)" onclick={() => api.newTask()}>＋</button>
      <button class="icon" title="Open editor" onclick={() => api.openEditor()}>↗</button>
    </div>
  </header>

  <section class="quick-add">
    <div class="qa-row">
      <input
        bind:this={titleInput}
        type="text"
        placeholder="New task…"
        bind:value={qTitle}
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            quickAdd();
          }
        }}
      />
      <select
        class="list-select"
        aria-label="Task list"
        bind:value={listId}
        disabled={lists.length === 0}
      >
        {#if lists.length === 0}
          <option value="">No lists yet</option>
        {:else}
          {#each lists as list (list.id)}
            <option value={list.id}>{list.title}</option>
          {/each}
        {/if}
      </select>
    </div>
    <div class="qa-footer">
      {#if lists.length === 0}
        <span class="qa-hint">Create a task list in the editor first.</span>
      {/if}
      <button class="primary add-btn" onclick={quickAdd} disabled={busy || !listId || !qTitle.trim()}>
        {busy ? "Adding…" : "Add task"}
      </button>
    </div>
  </section>

  <section class="search">
    <input
      bind:this={searchInput}
      type="search"
      placeholder="Search tasks…  (Ctrl+F)"
      bind:value={query}
      oninput={() => refresh()}
    />
  </section>

  <section class="list">
    {#if visibleTasks.length === 0}
      <div class="empty">
        {pendingOnly
          ? "No pending tasks."
          : query.trim()
            ? "No tasks match your search."
            : "No tasks yet. Add your first one above."}
      </div>
    {:else}
      {#each visibleTasks as task (task.id)}
        <TaskListItem
          {task}
          listTitle={listTitleById.get(task.listId)}
          onOpen={openInEditor}
          onToggle={toggleTask}
          onDelete={removeTask}
        />
      {/each}
    {/if}
  </section>

  <footer>
    <span class="hint"><kbd>Ctrl</kbd>+<kbd>N</kbd> new · <kbd>Ctrl</kbd>+<kbd>F</kbd> search · <kbd>Esc</kbd> hide</span>
  </footer>
</main>

<style>
  .popover {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    color: var(--text);
  }

  .logo {
    font-size: 16px;
  }

  .header-actions {
    display: flex;
    gap: 2px;
  }

  .quick-add {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
  }

  .qa-row {
    display: flex;
    gap: 8px;
  }

  .qa-row input {
    flex: 1;
  }

  .list-select {
    font-family: inherit;
    font-size: 13px;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 8px;
    outline: none;
    max-width: 130px;
  }

  .list-select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .qa-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .qa-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .add-btn {
    align-self: flex-end;
  }

  .search {
    padding: 8px 12px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 28px 12px;
    font-size: 13px;
  }

  footer {
    padding: 7px 12px;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
  }
</style>
