// app/lib/api.ts
// Thin typed wrapper around the Tauri commands. Also provides a tiny
// in-memory fallback so the UI can be developed in a plain browser.

import type { AppSettings, ImportSummary, Task, TaskList } from "./types";

// True when running inside the Tauri webview.
export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const api = {
  // Task lists
  async listTaskLists(): Promise<TaskList[]> {
    return invoke<TaskList[]>("list_task_lists");
  },
  async getTaskList(id: string): Promise<TaskList | null> {
    return invoke<TaskList | null>("get_task_list", { id });
  },
  async createTaskList(title: string): Promise<TaskList> {
    return invoke<TaskList>("create_task_list", { title });
  },
  async updateTaskList(list: TaskList): Promise<TaskList> {
    return invoke<TaskList>("update_task_list", { list });
  },
  async deleteTaskList(id: string): Promise<boolean> {
    return invoke<boolean>("delete_task_list", { id });
  },

  // Tasks
  async listTasks(listId?: string): Promise<Task[]> {
    return invoke<Task[]>("list_tasks", { listId: listId ?? null });
  },
  async searchTasks(query: string): Promise<Task[]> {
    return invoke<Task[]>("search_tasks", { query });
  },
  async getTask(id: string): Promise<Task | null> {
    return invoke<Task | null>("get_task", { id });
  },
  async createTask(
    listId: string,
    title: string,
    link: string = "",
    comment: string = "",
  ): Promise<Task> {
    return invoke<Task>("create_task", { listId, title, link, comment });
  },
  async updateTask(task: Task): Promise<Task> {
    return invoke<Task>("update_task", { task });
  },
  async deleteTask(id: string): Promise<boolean> {
    return invoke<boolean>("delete_task", { id });
  },
  async toggleTask(id: string): Promise<Task> {
    return invoke<Task>("toggle_task", { id });
  },

  // Export / Import
  async exportBundle(path: string): Promise<void> {
    await invoke("export_bundle", { path });
  },
  async importBundle(path: string): Promise<ImportSummary> {
    return invoke<ImportSummary>("import_bundle", { path });
  },

  // Window / tray
  async hideTray(): Promise<void> {
    await invoke("hide_tray");
  },
  async openEditor(taskId?: string): Promise<void> {
    await invoke("open_editor", { taskId: taskId ?? null });
  },
  async newTask(): Promise<void> {
    await invoke("new_task");
  },
  async takePendingAction(): Promise<string | null> {
    return invoke<string | null>("take_pending_action");
  },
  async quit(): Promise<void> {
    await invoke("quit_app");
  },

  // Settings
  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>("get_settings");
  },
  async setSetting(key: string, value: boolean | string): Promise<AppSettings> {
    return invoke<AppSettings>("set_setting", { key, value: String(value) });
  },
};
