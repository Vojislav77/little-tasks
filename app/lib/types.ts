// app/lib/types.ts
// Shared frontend types (mirror the Rust `crate::core::task::{Task, TaskList}`).

export interface TaskList {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
}

export interface Task {
  id: string;
  listId: string;
  title: string;
  done: boolean;
  link: string;
  comment: string;
  createdAt: string;
  updatedAt: string;
}

export interface ImportSummary {
  totalLists: number;
  importedLists: number;
  updatedLists: number;
  totalTasks: number;
  importedTasks: number;
  updatedTasks: number;
  skippedNewerLocal: number;
  skippedInvalid: number;
}

export interface ExportResult {
  path: string;
  taskCount: number;
  listCount: number;
}

export interface AppSettings {
  startWithSystem: boolean;
  showPendingOnly: boolean;
}

export type PendingAction =
  | { type: "new" }
  | { type: "open"; taskId: string }
  | { type: "list"; listId: string }
  | { type: "none" };

export type TrayAction = "export" | "import";
