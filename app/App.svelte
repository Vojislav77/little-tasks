<script lang="ts">
  import { onMount } from "svelte";
  import TrayWindow from "./windows/TrayWindow.svelte";
  import EditorWindow from "./windows/EditorWindow.svelte";
  import Toasts from "./components/Toasts.svelte";
  import { isTauri } from "./lib/api";

  let kind = $state<"tray" | "editor">("editor");

  onMount(async () => {
    if (isTauri()) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      kind = getCurrentWindow().label === "tray" ? "tray" : "editor";
    }
  });
</script>

{#if kind === "tray"}
  <TrayWindow />
{:else}
  <EditorWindow />
{/if}
<Toasts />
