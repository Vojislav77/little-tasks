<script lang="ts">
  import type { Task } from "../lib/types";
  import { formatWhen } from "../lib/format";

  interface Props {
    task: Task;
    active?: boolean;
    /** Optional list name shown when rows mix multiple lists (search). */
    listTitle?: string;
    onOpen: (task: Task) => void;
    onToggle: (task: Task) => void;
    onDelete: (task: Task) => void;
  }

  let { task, active = false, listTitle, onOpen, onToggle, onDelete }: Props = $props();

  function handleToggle(e: MouseEvent) {
    e.stopPropagation();
    onToggle(task);
  }
</script>

<div
  class="task-item"
  class:active
  class:done={task.done}
  role="button"
  tabindex="0"
  onclick={() => onOpen(task)}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onOpen(task);
    }
  }}
  data-testid="task-item"
>
  <button
    class="check"
    class:checked={task.done}
    role="checkbox"
    aria-checked={task.done}
    aria-label={task.done ? "Mark as pending" : "Mark as done"}
    title={task.done ? "Mark as pending" : "Mark as done"}
    onclick={handleToggle}
  >
    {#if task.done}<span class="check-mark">✓</span>{/if}
  </button>

  <span class="task-main">
    <span class="task-title">{task.title || "Untitled"}</span>
    {#if task.link || task.comment}
      <span class="task-meta">
        {#if task.link}<span class="meta-chip" title="Has link">↗</span>{/if}
        {#if task.comment}<span class="meta-chip" title="Has comment">💬</span>{/if}
        {#if listTitle}<span class="list-tag">{listTitle}</span>{/if}
      </span>
    {:else if listTitle}
      <span class="task-meta"><span class="list-tag">{listTitle}</span></span>
    {/if}
    <span class="task-when">{formatWhen(task.updatedAt)}</span>
  </span>

  <span class="task-actions">
    <button
      class="icon"
      title="Open in editor"
      onclick={(e) => {
        e.stopPropagation();
        onOpen(task);
      }}
      >↗</button
    >
    <button
      class="icon danger"
      title="Delete task"
      onclick={(e) => {
        e.stopPropagation();
        onDelete(task);
      }}
      >🗑</button
    >
  </span>
</div>

<style>
  .task-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    text-align: left;
    border: 1px solid transparent;
    background: transparent;
    border-radius: 10px;
    padding: 8px;
    margin: 1px 0;
    cursor: pointer;
    color: var(--text);
  }

  .task-item:hover {
    background: var(--surface);
    border-color: var(--border);
  }

  .task-item.active {
    background: var(--accent-soft);
    border-color: var(--border-strong);
  }

  .task-item.done .task-title {
    color: var(--text-muted);
    text-decoration: line-through;
  }

  .check {
    flex: none;
    width: 18px;
    height: 18px;
    margin-top: 1px;
    border: 1.5px solid var(--border-strong);
    border-radius: 5px;
    background: var(--surface);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    cursor: pointer;
    color: #fff;
    transition: background 0.12s ease, border-color 0.12s ease;
  }

  .check:hover {
    border-color: var(--accent);
  }

  .check.checked {
    background: var(--accent);
    border-color: var(--accent);
  }

  .check-mark {
    font-size: 12px;
    line-height: 1;
  }

  .task-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .task-title {
    font-weight: 600;
    font-size: 13px;
    word-break: break-word;
  }

  .task-meta {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .meta-chip {
    color: var(--accent-strong);
  }

  .list-tag {
    background: var(--accent-soft);
    color: var(--accent-strong);
    border-radius: 999px;
    padding: 0 7px;
    font-size: 10px;
  }

  .task-when {
    font-size: 10px;
    color: var(--text-muted);
    opacity: 0.85;
  }

  .task-actions {
    display: none;
    flex-shrink: 0;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .task-item:hover .task-actions {
    display: inline-flex;
    opacity: 1;
  }
</style>
