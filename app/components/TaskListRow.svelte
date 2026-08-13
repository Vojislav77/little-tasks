<script lang="ts">
  import type { TaskList } from "../lib/types";
  import { focusOnMount } from "../lib/focus";

  interface Props {
    list: TaskList;
    taskCount?: number;
    active?: boolean;
    onSelect: (list: TaskList) => void;
    onRename: (list: TaskList, newTitle: string) => void;
    onDelete: (list: TaskList) => void;
  }

  let { list, taskCount = 0, active = false, onSelect, onRename, onDelete }: Props = $props();

  let editing = $state(false);
  // svelte-ignore state_referenced_locally
  let draft = $state(list.title);

  function startEdit() {
    draft = list.title;
    editing = true;
  }

  function commitEdit() {
    editing = false;
    const title = draft.trim();
    if (title && title !== list.title) onRename(list, title);
  }

  function cancelEdit() {
    editing = false;
    draft = list.title;
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      commitEdit();
    } else if (e.key === "Escape") {
      cancelEdit();
    }
  }
</script>

{#if editing}
  <input
    class="list-edit-input"
    type="text"
    value={draft}
    use:focusOnMount
    oninput={(e) => (draft = (e.currentTarget as HTMLInputElement).value)}
    onkeydown={handleKey}
    onblur={commitEdit}
  />
{:else}
  <div
    class="list-row"
    class:active
    role="button"
    tabindex="0"
    onclick={() => onSelect(list)}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelect(list);
      }
    }}
    data-testid="list-row"
  >
    <span class="list-title" title={list.title}>{list.title || "Untitled"}</span>
    <span class="list-count">{taskCount}</span>
    <span class="list-actions">
      <button
        class="icon"
        title="Rename list"
        onclick={(e) => {
          e.stopPropagation();
          startEdit();
        }}
        >✎</button
      >
      <button
        class="icon danger"
        title="Delete list"
        onclick={(e) => {
          e.stopPropagation();
          onDelete(list);
        }}
        >🗑</button
      >
    </span>
  </div>
{/if}

<style>
  .list-row {
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

  .list-row:hover {
    background: var(--surface);
    border-color: var(--border);
  }

  .list-row.active {
    background: var(--accent-soft);
    border-color: var(--border-strong);
  }

  .list-title {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .list-count {
    flex: none;
    font-size: 11px;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 7px;
    min-width: 20px;
    text-align: center;
  }

  .list-actions {
    display: none;
    flex-shrink: 0;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .list-row:hover .list-actions {
    display: inline-flex;
    opacity: 1;
  }

  .list-edit-input {
    margin: 1px 0;
    padding: 6px 8px;
    font-size: 13px;
    font-weight: 600;
    background: var(--surface);
    border: 1px solid var(--accent);
    border-radius: 8px;
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
</style>
