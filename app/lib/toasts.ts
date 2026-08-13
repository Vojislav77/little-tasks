// app/lib/toasts.ts
// Minimal toast store.

import { writable } from "svelte/store";

export type ToastKind = "success" | "error" | "info";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

export const toasts = writable<Toast[]>([]);

let nextId = 1;

export function toast(message: string, kind: ToastKind = "info", ttl = 4000): void {
  const id = nextId++;
  toasts.update((t) => [...t, { id, kind, message }]);
  setTimeout(() => {
    toasts.update((t) => t.filter((x) => x.id !== id));
  }, ttl);
}

export const toastSuccess = (m: string) => toast(m, "success");
export const toastError = (m: string) => toast(m, "error", 7000);
export const toastInfo = (m: string) => toast(m, "info");
