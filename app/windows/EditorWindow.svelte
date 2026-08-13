<script lang="ts">
  import { onMount } from "svelte";
  import TaskListItem from "../components/TaskListItem.svelte";
  import TaskListRow from "../components/TaskListRow.svelte";
  import { api, isTauri } from "../lib/api";
  import type { AppSettings, Task, TaskList } from "../lib/types";
  import { firstLine } from "../lib/format";
  import { focusOnMount } from "../lib/focus";
  import { toastError, toastSuccess } from "../lib/toasts";

  let lists = $state<TaskList[]>([]);
  let allTasks = $state<Task[]>([]);
  let query = $state("");
  let pendingOnly = $state(false);
  let selectedListId = $state<string | null>(null);
  let selectedTaskId = $state<string | null>(null);
  let newTitle = $state("");
  let addingList = $state(false);
  let newListTitle = $state("");
  let saving = $state(false);
  let appVersion = $state("");
  let settingsOpen = $state(false);
  let startWithSystem = $state(false);

  let draft = $state({
    id: "",
    listId: "",
    title: "",
    done: false,
    link: "",
    comment: "",
    updatedAt: "",
  });

  let searchInput = $state<HTMLInputElement | null>(null);
  let newTaskInput = $state<HTMLInputElement | null>(null);

  const listTitleById = $derived(
    new Map(lists.map((l) => [l.id, l.title])),
  );

  const listTaskCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const t of allTasks) {
      counts.set(t.listId, (counts.get(t.listId) ?? 0) + 1);
    }
    return counts;
  });

  const selectedTask = $derived(
    allTasks.find((t) => t.id === selectedTaskId) ?? null,
  );

  const viewTitle = $derived.by(() => {
    if (query.trim()) return "Search results";
    if (!selectedListId) return "All tasks";
    return listTitleById.get(selectedListId) ?? "Untitled list";
  });

  const visibleTasks = $derived.by(() => {
    let tasks = allTasks;
    if (query.trim()) {
      return pendingOnly ? tasks.filter((t) => !t.done) : tasks;
    }
    if (selectedListId) tasks = tasks.filter((t) => t.listId === selectedListId);
    return pendingOnly ? tasks.filter((t) => !t.done) : tasks;
  });

  const isDirty = $derived.by(() => {
    if (!selectedTask) return draft.title !== "" || draft.link !== "" || draft.comment !== "";
    return (
      draft.title !== selectedTask.title ||
      draft.link !== selectedTask.link ||
      draft.comment !== selectedTask.comment ||
      draft.done !== selectedTask.done
    );
  });

  async function refresh() {
    try {
      allTasks = query.trim() ? await api.searchTasks(query) : await api.listTasks();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function refreshLists() {
    try {
      lists = await api.listTaskLists();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function loadSettings() {
    try {
      const settings = await api.getSettings();
      pendingOnly = settings.showPendingOnly;
      startWithSystem = settings.startWithSystem;
    } catch (e) {
      toastError(String(e));
    }
  }

  function resetDraft() {
    draft.id = "";
    draft.listId = "";
    draft.title = "";
    draft.done = false;
    draft.link = "";
    draft.comment = "";
    draft.updatedAt = "";
    selectedTaskId = null;
  }

  function selectTask(task: Task) {
    selectedTaskId = task.id;
    draft.id = task.id;
    draft.listId = task.listId;
    draft.title = task.title;
    draft.done = task.done;
    draft.link = task.link;
    draft.comment = task.comment;
    draft.updatedAt = task.updatedAt;
  }

  function selectList(list: TaskList) {
    if (isDirty && selectedTaskId && !confirm("Discard unsaved changes?")) return;
    selectedListId = list.id;
    resetDraft();
  }

  function showAll() {
    if (isDirty && selectedTaskId && !confirm("Discard unsaved changes?")) return;
    selectedListId = null;
    resetDraft();
  }

  async function addTask() {
    const title = newTitle.trim();
    const target = selectedListId ?? lists[0]?.id;
    if (!title || !target) return;
    try {
      await api.createTask(target, title);
      newTitle = "";
      await refresh();
      toastSuccess("Task added");
      newTaskInput?.focus();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function saveTask() {
    if (saving || !selectedTask) return;
    const title = draft.title.trim() || firstLine(draft.comment) || "Untitled";
    saving = true;
    try {
      const updated = await api.updateTask({
        id: selectedTask.id,
        listId: selectedTask.listId,
        title,
        done: draft.done,
        link: draft.link,
        comment: draft.comment,
        createdAt: selectedTask.createdAt,
        updatedAt: selectedTask.updatedAt,
      });
      draft.updatedAt = updated.updatedAt;
      await refresh();
      toastSuccess("Saved");
    } catch (e) {
      toastError(String(e));
    } finally {
      saving = false;
    }
  }

  async function toggleFromList(task: Task) {
    try {
      await api.toggleTask(task.id);
      if (selectedTaskId === task.id) draft.done = !task.done;
      await refresh();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function deleteSelected() {
    if (!selectedTask) return;
    const confirmed =
      (await confirmDialog(`Delete "${draft.title || "Untitled"}"? This cannot be undone.`)) === true;
    if (!confirmed) return;
    try {
      await api.deleteTask(selectedTask.id);
      resetDraft();
      await refresh();
      toastSuccess("Task deleted");
    } catch (e) {
      toastError(String(e));
    }
  }

  async function deleteFromList(task: Task) {
    const confirmed =
      (await confirmDialog(`Delete "${task.title || "Untitled"}"? This cannot be undone.`)) === true;
    if (!confirmed) return;
    try {
      await api.deleteTask(task.id);
      if (selectedTaskId === task.id) resetDraft();
      await refresh();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function createList() {
    const title = newListTitle.trim();
    if (!title) return;
    try {
      const created = await api.createTaskList(title);
      newListTitle = "";
      addingList = false;
      await refreshLists();
      await refresh();
      selectedListId = created.id;
      resetDraft();
      toastSuccess("List created");
    } catch (e) {
      toastError(String(e));
    }
  }

  async function renameList(list: TaskList, newTitle: string) {
    try {
      await api.updateTaskList({ ...list, title: newTitle });
      await refreshLists();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function deleteList(list: TaskList) {
    const confirmed =
      (await confirmDialog(
        `Delete list "${list.title}" and all its tasks? This cannot be undone.`,
      )) === true;
    if (!confirmed) return;
    try {
      await api.deleteTaskList(list.id);
      if (selectedListId === list.id) {
        selectedListId = null;
        resetDraft();
      }
      if (draft.listId === list.id) resetDraft();
      await refreshLists();
      await refresh();
    } catch (e) {
      toastError(String(e));
    }
  }

  function newTaskShortcut() {
    if (isDirty && selectedTaskId && !confirm("Discard unsaved changes?")) return;
    resetDraft();
    newTitle = "";
    newTaskInput?.focus();
  }

  async function toggleStartWithSystem(e: Event) {
    const next = (e.currentTarget as HTMLInputElement).checked;
    const previous = startWithSystem;
    startWithSystem = next;
    try {
      await api.setSetting("start_with_system", next);
    } catch (err) {
      startWithSystem = previous;
      toastError(String(err));
    }
  }

  async function togglePendingOnly(e: Event) {
    const next = (e.currentTarget as HTMLInputElement).checked;
    const previous = pendingOnly;
    pendingOnly = next;
    try {
      await api.setSetting("show_pending_only", next);
    } catch (err) {
      pendingOnly = previous;
      toastError(String(err));
    }
  }

  async function confirmDialog(message: string): Promise<boolean> {
    if (isTauri()) {
      const { ask } = await import("@tauri-apps/plugin-dialog");
      return ask(message, { title: "Little Tasks", kind: "warning" });
    }
    return window.confirm(message);
  }

  async function onExport() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const stamp = new Date().toISOString().slice(0, 10);
      const path = await save({
        defaultPath: `little-tasks-${stamp}.json`,
        filters: [{ name: "JSON bundle", extensions: ["json"] }],
      });
      if (!path) return;
      await api.exportBundle(path);
      toastSuccess(`Exported to ${path}`);
    } catch (e) {
      toastError(`Export failed: ${e}`);
    }
  }

  async function onImport() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "JSON bundle", extensions: ["json"] }],
      });
      if (!picked || Array.isArray(picked)) return;
      const summary = await api.importBundle(picked);
      await refreshLists();
      await refresh();
      toastSuccess(
        `Import complete — ${summary.importedLists} lists, ${summary.importedTasks} new tasks, ` +
          `${summary.updatedTasks} updated, ${summary.skippedNewerLocal} kept (newer), ` +
          `${summary.skippedInvalid} skipped (invalid)`,
      );
    } catch (e) {
      toastError(`Import failed: ${e}`);
    }
  }

  function parsePending(raw: string | null): "new" | "task" | "list" | "default" {
    if (raw === "__new__") return "new";
    if (raw && raw.startsWith("task:")) return "task";
    if (raw && raw.startsWith("list:")) return "list";
    return "default";
  }

  onMount(() => {
    let dispose: (() => void) | undefined;
    void (async () => {
      await refreshLists();
      await refresh();
      await loadSettings();

      if (isTauri()) {
        try {
          const { getVersion } = await import("@tauri-apps/api/app");
          appVersion = await getVersion();
        } catch {
          appVersion = "unknown";
        }

        const { listen } = await import("@tauri-apps/api/event");
        const unlisten = await listen("tasks-changed", () => {
          void refreshLists();
          void refresh();
        });
        const unlistenSettings = await listen<AppSettings>("settings-changed", (e) => {
          startWithSystem = e.payload.startWithSystem;
          pendingOnly = e.payload.showPendingOnly;
        });

        const handleKey = (ev: KeyboardEvent) => {
          if (ev.ctrlKey && !ev.shiftKey && !ev.altKey && !ev.metaKey) {
            if (ev.key === "n" || ev.key === "N") {
              ev.preventDefault();
              newTaskShortcut();
            } else if (ev.key === "f" || ev.key === "F") {
              ev.preventDefault();
              searchInput?.focus();
              searchInput?.select();
            } else if (ev.key === "s" || ev.key === "S") {
              ev.preventDefault();
              if (selectedTaskId && isDirty) {
                void saveTask();
              } else if (newTitle.trim()) {
                void addTask();
              }
            } else if (ev.key === "q" || ev.key === "Q") {
              ev.preventDefault();
              api.quit();
            }
          }
        };
        window.addEventListener("keydown", handleKey);

        // Claim pending action left by the tray.
        const pending = await api.takePendingAction();
        const kind = parsePending(pending);
        if (kind === "new") {
          newTitle = "";
          newTaskInput?.focus();
        } else if (kind === "task" && pending) {
          const taskId = pending.slice("task:".length);
          const task = allTasks.find((t) => t.id === taskId) ?? (await api.getTask(taskId));
          if (task) {
            selectedListId = task.listId;
            await refresh();
            selectTask(task);
          }
        } else if (kind === "list" && pending) {
          const listId = pending.slice("list:".length);
          if (lists.some((l) => l.id === listId)) selectedListId = listId;
        }

        dispose = () => {
          window.removeEventListener("keydown", handleKey);
          unlisten();
          unlistenSettings();
        };
      }
    })();
    return () => dispose?.();
  });
</script>

<div class="editor">
  <!-- Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-head">
      <span class="brand">📌 Little Tasks</span>
      <div class="head-actions">
        <button class="icon icon-lg" title="New task list" onclick={() => (addingList = !addingList)}>＋</button>
        <button class="icon icon-lg" title="Settings" onclick={() => (settingsOpen = true)}>⚙</button>
      </div>
    </div>

    <div class="search">
      <input
        bind:this={searchInput}
        type="search"
        placeholder="Search tasks…  (Ctrl+F)"
        bind:value={query}
        oninput={() => refresh()}
      />
    </div>

    {#if addingList}
      <div class="new-list">
        <input
          type="text"
          placeholder="List name…"
          bind:value={newListTitle}
          use:focusOnMount
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void createList();
            } else if (e.key === "Escape") {
              addingList = false;
              newListTitle = "";
            }
          }}
          onblur={() => {
            addingList = false;
          }}
        />
      </div>
    {/if}

    <div class="list">
      <div
        class="all-row"
        class:active={!selectedListId && !query.trim()}
        role="button"
        tabindex="0"
        onclick={showAll}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            showAll();
          }
        }}
      >
        <span class="all-title">All tasks</span>
        <span class="list-count">{allTasks.length}</span>
      </div>

      {#if lists.length === 0}
        <div class="empty">No lists yet. Press ＋ to create one.</div>
      {:else}
        {#each lists as list (list.id)}
          <TaskListRow
            {list}
            taskCount={listTaskCounts.get(list.id) ?? 0}
            active={list.id === selectedListId}
            onSelect={selectList}
            onRename={renameList}
            onDelete={deleteList}
          />
        {/each}
      {/if}
    </div>
  </aside>

  <!-- Editor pane -->
  <section class="pane">
    <div class="toolbar">
      <div class="toolbar-left">
        <button
          class="primary save-btn"
          onclick={saveTask}
          disabled={saving || !selectedTask || !isDirty}
        >
          {saving ? "Saving…" : "Save"}
          {#if selectedTask}<span class="shortcut"><kbd>Ctrl</kbd>+<kbd>S</kbd></span>{/if}
        </button>
        <button class="icon danger" title="Delete task" onclick={deleteSelected} disabled={!selectedTask}>
          🗑
        </button>
        <span class="sep"></span>
        <span class="view-title">{viewTitle}</span>
      </div>
      <div class="toolbar-right">
        <span class="dirty-dot" class:shown={isDirty} title="Unsaved changes"></span>
      </div>
    </div>

    <div class="compose">
      <input
        bind:this={newTaskInput}
        type="text"
        placeholder="Add a task…  (Ctrl+N)"
        bind:value={newTitle}
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void addTask();
          }
        }}
      />
      <button class="primary" onclick={addTask} disabled={!newTitle.trim() || lists.length === 0}>
        Add
      </button>
    </div>
    {#if lists.length === 0}
      <div class="compose-hint">Create a task list first (＋ in the sidebar).</div>
    {/if}

    <div class="task-list">
      {#if visibleTasks.length === 0}
        <div class="empty">
          {pendingOnly && !query.trim()
            ? "No pending tasks here."
            : query.trim()
              ? "No tasks match your search."
              : "No tasks yet. Add your first one above."}
        </div>
      {:else}
        {#each visibleTasks as task (task.id)}
          <TaskListItem
            {task}
            active={task.id === selectedTaskId}
            listTitle={query.trim() ? listTitleById.get(task.listId) : undefined}
            onOpen={selectTask}
            onToggle={toggleFromList}
            onDelete={deleteFromList}
          />
        {/each}
      {/if}
    </div>

    {#if selectedTask}
      <section class="details">
        <div class="details-head">
          <span class="details-label">Task details</span>
          <label class="done-toggle">
            <input type="checkbox" bind:checked={draft.done} />
            <span>Done</span>
          </label>
        </div>
        <input
          type="text"
          class="details-title"
          placeholder="Task title"
          bind:value={draft.title}
          onkeydown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              void saveTask();
            }
          }}
        />
        <label class="field">
          <span class="field-label">Link</span>
          <input
            type="text"
            placeholder="https://…"
            spellcheck="false"
            bind:value={draft.link}
          />
        </label>
        <label class="field">
          <span class="field-label">Comment</span>
          <textarea
            rows="3"
            placeholder="Notes about this task…"
            bind:value={draft.comment}
          ></textarea>
        </label>
      </section>
    {/if}

    <footer class="app-footer">
      <span class="copyright">© 2026 Vojislav Korać</span>
      <span class="footer-version">v{appVersion || "0.1.0"}</span>
    </footer>
  </section>

  {#if settingsOpen}
    <div
      class="modal-backdrop"
      role="presentation"
      onclick={(e) => {
        if (e.target === e.currentTarget) settingsOpen = false;
      }}
    >
      <div class="modal" role="dialog" aria-modal="true" aria-label="Settings">
        <div class="modal-head">
          <span>Settings</span>
          <button class="icon" title="Close" onclick={() => (settingsOpen = false)}>✕</button>
        </div>
        <label class="setting-row">
          <div class="setting-text">
            <div class="setting-name">Start with system</div>
            <div class="setting-desc">Launch Little Tasks automatically when you log in.</div>
          </div>
          <input type="checkbox" checked={startWithSystem} onchange={toggleStartWithSystem} />
        </label>
        <label class="setting-row">
          <div class="setting-text">
            <div class="setting-name">Show only pending tasks</div>
            <div class="setting-desc">Hide completed tasks from the lists.</div>
          </div>
          <input type="checkbox" checked={pendingOnly} onchange={togglePendingOnly} />
        </label>

        <div class="modal-section-title">Data</div>
        <div class="data-actions">
          <button class="primary" onclick={onExport}>Export JSON</button>
          <button onclick={onImport}>Import JSON</button>
        </div>

        <div class="modal-foot">
          <button class="primary" onclick={() => (settingsOpen = false)}>Done</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor {
    display: flex;
    height: 100vh;
    background: var(--bg);
  }

  .sidebar {
    width: 260px;
    min-width: 260px;
    display: flex;
    flex-direction: column;
    background: var(--surface-2);
    border-right: 1px solid var(--border);
  }

  .sidebar-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 8px;
  }

  .head-actions {
    display: flex;
    gap: 2px;
  }

  .head-actions .icon-lg {
    font-size: 25px;
    padding: 2px 6px;
    line-height: 1;
  }

  .brand {
    font-weight: 700;
  }

  .search {
    padding: 4px 12px 8px;
  }

  .new-list {
    padding: 0 12px 8px;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
  }

  .all-row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    text-align: left;
    border: 1px solid transparent;
    background: transparent;
    border-radius: 8px;
    padding: 7px 8px;
    margin: 1px 0;
    cursor: pointer;
    color: var(--text);
  }

  .all-row:hover {
    background: var(--surface);
    border-color: var(--border);
  }

  .all-row.active {
    background: var(--accent-soft);
    border-color: var(--border-strong);
  }

  .all-title {
    flex: 1;
    font-weight: 700;
    font-size: 13px;
  }

  .list-count {
    flex: none;
    font-size: 11px;
    color: var(--text-muted);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
    min-width: 20px;
    text-align: center;
  }

  .empty {
    color: var(--text-muted);
    font-size: 13px;
    text-align: center;
    padding: 20px 12px;
  }

  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface);
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .save-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .shortcut {
    font-weight: 400;
    opacity: 0.85;
  }

  .sep {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 6px;
  }

  .view-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dirty-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--danger);
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .dirty-dot.shown {
    opacity: 1;
  }

  .compose {
    display: flex;
    gap: 8px;
    padding: 10px 14px 4px;
  }

  .compose-hint {
    padding: 4px 16px 8px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .task-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 10px 10px;
  }

  .details {
    flex: none;
    max-height: 46%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .details-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .details-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .done-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
    user-select: none;
  }

  .done-toggle input[type="checkbox"] {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .details-title {
    font-size: 18px;
    font-weight: 700;
    border: none;
    background: transparent;
    padding: 2px;
    box-shadow: none !important;
  }

  .details-title:focus {
    border: none;
    box-shadow: none;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .field textarea {
    font-family: inherit;
    font-size: 13px;
  }

  .app-footer {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 18px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg);
  }

  .copyright {
    font-size: 12px;
    color: var(--text-muted);
  }

  .footer-version {
    font-size: 12px;
    color: var(--text-muted);
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(30, 50, 70, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }

  .modal {
    width: 400px;
    max-width: calc(100vw - 40px);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    padding: 16px;
  }

  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-weight: 700;
    font-size: 15px;
    margin-bottom: 8px;
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 4px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    user-select: none;
  }

  .setting-row:last-of-type {
    border-bottom: none;
  }

  .setting-name {
    font-weight: 600;
    font-size: 14px;
    color: var(--text);
  }

  .setting-desc {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .setting-row input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
    flex: none;
    cursor: pointer;
  }

  .modal-section-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    padding: 14px 4px 6px;
  }

  .data-actions {
    display: flex;
    gap: 8px;
    padding: 0 4px;
  }

  .modal-foot {
    display: flex;
    justify-content: flex-end;
    margin-top: 14px;
  }
</style>
