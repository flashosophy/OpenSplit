<script lang="ts">
  import { onMount } from "svelte";
  import PaneView from "./PaneView.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import LauncherPicker from "./LauncherPicker.svelte";
  import SettingsPanel from "./SettingsPanel.svelte";
  import {
    closePane,
    getStartupAction,
    resolveSplitSpec,
    setDefaultProfile,
    spawnPane,
    type DetectedTool,
    type LaunchSpec,
    type SpawnSource,
    type StartupAction,
  } from "../lib/ipc";
  import { copyText, pasteText } from "../lib/clipboard";
  import { getTerminal } from "../lib/terminalRegistry";
  import {
    findLeafByPaneId,
    leaves,
    makeLeaf,
    removeLeaf,
    setRatio,
    splitLeaf,
    type PaneNode,
    type SplitDirection,
  } from "../lib/PaneTree";

  let tree = $state<PaneNode | null>(null);
  let focusedPaneId = $state<string | null>(null);
  let booting = $state(true);
  let bootError = $state<string | null>(null);

  /** When non-null, we're showing the launcher picker instead of a tree. */
  let pickerTools = $state<DetectedTool[] | null>(null);

  /** Settings panel visibility (overlay; works in either picker or tree state). */
  let showSettings = $state(false);

  let ctxMenu = $state<{
    x: number;
    y: number;
    paneId: string;
    hasSelection: boolean;
  } | null>(null);

  const INITIAL_COLS = 100;
  const INITIAL_ROWS = 30;

  onMount(async () => {
    try {
      const action: StartupAction = await getStartupAction();
      if (action.kind === "launch") {
        await spawnFromSpec(action.spec);
      } else {
        pickerTools = action.detected;
      }
    } catch (e) {
      bootError = String(e);
      console.error("[opensplit] boot failed", e);
    } finally {
      booting = false;
    }
  });

  async function spawnFromSource(source: SpawnSource, title: string) {
    const result = await spawnPane(source, INITIAL_COLS, INITIAL_ROWS);
    tree = makeLeaf(result.pane_id, result.spec.profile, title);
    focusedPaneId = result.pane_id;
  }

  async function spawnFromSpec(spec: LaunchSpec) {
    const title = spec.profile ?? spec.command;
    await spawnFromSource({ kind: "spec", spec }, title);
  }

  async function pickFromLauncher(tool: DetectedTool, setAsDefault: boolean) {
    try {
      if (setAsDefault) {
        try {
          await setDefaultProfile(tool.name);
        } catch (e) {
          console.warn("[opensplit] could not save default", e);
        }
      }
      pickerTools = null;
      await spawnFromSource({ kind: "detected", name: tool.name }, tool.label);
    } catch (e) {
      bootError = `Failed to launch ${tool.label}: ${e}`;
      console.error("[opensplit] picker launch failed", e);
      // Re-show picker so the user can pick something else.
      pickerTools = pickerTools ?? [];
    }
  }

  function focusPane(paneId: string) {
    focusedPaneId = paneId;
  }

  function openContextMenu(ev: MouseEvent, paneId: string) {
    ev.preventDefault();
    ev.stopPropagation();
    focusedPaneId = paneId;
    // Capture selection at menu-open time. If the user right-clicks while
    // text is selected, Copy should be enabled even though clicking the
    // menu item happens after focus shifts.
    const t = getTerminal(paneId);
    const hasSelection = !!t && t.getSelection().length > 0;
    ctxMenu = { x: ev.clientX, y: ev.clientY, paneId, hasSelection };
  }

  function closeContextMenu() {
    ctxMenu = null;
  }

  async function performSplit(sourcePaneId: string, direction: SplitDirection) {
    if (!tree) return;
    const sourceLeaf = findLeafByPaneId(tree, sourcePaneId);
    if (!sourceLeaf) return;

    try {
      const resolved = await resolveSplitSpec(sourcePaneId, sourceLeaf.profile);
      const spawned = await spawnPane(
        { kind: "spec", spec: resolved.spec },
        INITIAL_COLS,
        INITIAL_ROWS,
      );
      const title = resolved.inherited_ssh
        ? `ssh: ${resolved.source_foreground?.cmd.slice(1).join(" ") ?? ""}`
        : (resolved.spec.profile ?? resolved.spec.command);
      const newLeaf = makeLeaf(
        spawned.pane_id,
        resolved.spec.profile,
        title,
      );
      tree = splitLeaf(tree, sourceLeaf.id, direction, newLeaf);
      focusedPaneId = spawned.pane_id;
    } catch (e) {
      console.error("[opensplit] split failed", e);
    }
  }

  async function performClose(paneId: string) {
    if (!tree) return;
    try {
      await closePane(paneId);
    } catch (e) {
      console.warn("[opensplit] close_pane error", e);
    }
    const leaf = findLeafByPaneId(tree, paneId);
    if (!leaf) return;
    const next = removeLeaf(tree, leaf.id);
    tree = next;
    if (next === null) {
      // Last pane closed → return to picker instead of closing the window.
      // This is friendlier when the user opened the app intentionally.
      try {
        const action = await getStartupAction();
        if (action.kind === "picker") {
          pickerTools = action.detected;
        } else {
          // Default is set; relaunch it.
          await spawnFromSpec(action.spec);
        }
      } catch (e) {
        console.error("[opensplit] post-close re-init failed", e);
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().close();
      }
      return;
    }
    const remaining = leaves(next);
    focusedPaneId = remaining[0]?.paneId ?? null;
  }

  function onSplitterDrag(splitId: string, ratio: number) {
    if (!tree) return;
    tree = setRatio(tree, splitId, ratio);
  }

  /**
   * Copy the focused pane's xterm selection to the system clipboard.
   * Falls back silently when nothing is selected — that's what every terminal
   * emulator does. The caller decides whether to also send Ctrl+C to the PTY
   * (no, we explicitly don't, because the user pressed Ctrl+SHIFT+C precisely
   * to avoid interrupting the running program).
   */
  async function performCopy(paneId: string): Promise<boolean> {
    const t = getTerminal(paneId);
    if (!t) return false;
    const sel = t.getSelection();
    if (!sel) return false;
    try {
      await copyText(sel);
      return true;
    } catch (e) {
      console.warn("[opensplit] clipboard write failed", e);
      return false;
    }
  }

  /**
   * Read the system clipboard and stuff it into the focused pane's PTY.
   * Uses xterm's `paste()` which emits bracketed-paste markers when the
   * remote app supports them — so multi-line clipboard content won't
   * auto-execute in a shell that's bracketed-paste-aware.
   */
  async function performPaste(paneId: string): Promise<boolean> {
    const t = getTerminal(paneId);
    if (!t) return false;
    let text: string;
    try {
      text = await pasteText();
    } catch (e) {
      console.warn("[opensplit] clipboard read failed", e);
      return false;
    }
    if (!text) return false;
    t.paste(text);
    return true;
  }

  function onKeydown(ev: KeyboardEvent) {
    // Ctrl+, opens settings (common convention).
    if (ev.ctrlKey && ev.key === ",") {
      ev.preventDefault();
      showSettings = true;
      return;
    }
    if (!focusedPaneId) return;
    const mod = ev.ctrlKey && ev.shiftKey;
    if (!mod) return;

    // Normalize to lowercase since Shift makes ev.key uppercase on letters.
    const k = ev.key.toLowerCase();

    // IMPORTANT: preventDefault + stopPropagation BEFORE awaiting anything,
    // so xterm's own keydown listener on the focused pane doesn't also see
    // the keystroke and send it to the PTY as e.g. ^C.
    //
    // Shortcut map (Ctrl+Shift+...):
    //   C            copy selection
    //   V            paste
    //   H or -       split with horizontal divider (panes stacked)
    //   E or |       split with vertical divider   (panes side by side)
    //   W            close focused pane
    // Note: Ctrl+Shift+V used to be "split vertical". It now pastes, which is
    // the universal terminal-emulator convention. Use E (or |) for vertical.
    if (k === "c") {
      ev.preventDefault();
      ev.stopPropagation();
      void performCopy(focusedPaneId);
    } else if (k === "v") {
      ev.preventDefault();
      ev.stopPropagation();
      void performPaste(focusedPaneId);
    } else if (k === "h" || ev.key === "_" || ev.key === "-") {
      ev.preventDefault();
      performSplit(focusedPaneId, "v"); // tree-direction "v" = horizontal divider, stacked
    } else if (k === "e" || ev.key === "|" || ev.key === "\\") {
      ev.preventDefault();
      performSplit(focusedPaneId, "h"); // tree-direction "h" = vertical divider, side by side
    } else if (k === "w") {
      ev.preventDefault();
      performClose(focusedPaneId);
    }
  }
</script>

<svelte:window on:keydown={onKeydown} on:click={closeContextMenu} />

<main class="root">
  {#if booting}
    <div class="boot">Starting OpenSplit…</div>
  {:else if bootError && !pickerTools}
    <div class="boot error">
      <h2>Failed to start</h2>
      <pre>{bootError}</pre>
      <button onclick={() => location.reload()}>Retry</button>
    </div>
  {:else if pickerTools !== null}
    <LauncherPicker detected={pickerTools} onPick={pickFromLauncher} />
  {:else if tree}
    <PaneView
      node={tree}
      {focusedPaneId}
      onFocus={focusPane}
      onContextMenu={openContextMenu}
      onSplitterDrag={onSplitterDrag}
    />
  {/if}

  <!-- Floating gear (top-right). Always visible once boot finishes. -->
  {#if !booting}
    <button
      class="gear"
      type="button"
      title="Settings (Ctrl+,)"
      aria-label="Settings"
      onclick={() => (showSettings = true)}
    >
      <svg viewBox="0 0 24 24" width="18" height="18">
        <path
          d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zm8.4 3.5c0 .5-.1 1-.2 1.5l2.1 1.6-2 3.4-2.5-.9c-.7.6-1.6 1-2.5 1.4l-.4 2.6h-4l-.4-2.6c-.9-.3-1.7-.8-2.5-1.4l-2.5.9-2-3.4 2.1-1.6c-.1-.5-.2-1-.2-1.5s.1-1 .2-1.5L3.5 8.9l2-3.4 2.5.9c.8-.6 1.6-1.1 2.5-1.4L10.9 2h4l.4 2.6c.9.3 1.8.8 2.5 1.4l2.5-.9 2 3.4-2.1 1.6c.1.5.2 1 .2 1.5z"
          fill="none" stroke="currentColor" stroke-width="1.3"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  {/if}

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} />
  {/if}

  {#if ctxMenu}
    <ContextMenu
      x={ctxMenu.x}
      y={ctxMenu.y}
      hasSelection={ctxMenu.hasSelection}
      onCopy={() => {
        const p = ctxMenu!.paneId;
        closeContextMenu();
        void performCopy(p);
      }}
      onPaste={() => {
        const p = ctxMenu!.paneId;
        closeContextMenu();
        void performPaste(p);
      }}
      onSplitHorizontal={() => {
        const p = ctxMenu!.paneId;
        closeContextMenu();
        performSplit(p, "v");
      }}
      onSplitVertical={() => {
        const p = ctxMenu!.paneId;
        closeContextMenu();
        performSplit(p, "h");
      }}
      onClose={() => {
        const p = ctxMenu!.paneId;
        closeContextMenu();
        performClose(p);
      }}
    />
  {/if}
</main>

<style>
  .root {
    position: fixed;
    inset: 0;
    background: var(--bg);
    display: flex;
    flex-direction: column;
  }
  .boot {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-dim);
    font-size: 14px;
  }
  .boot.error {
    flex-direction: column;
    color: var(--danger);
    padding: 24px;
    text-align: left;
    align-items: flex-start;
    gap: 12px;
  }
  .boot.error pre {
    background: var(--bg-elev);
    padding: 12px;
    border-radius: 4px;
    color: var(--fg);
    max-width: 100%;
    overflow: auto;
    white-space: pre-wrap;
  }

  .gear {
    position: fixed;
    top: 8px;
    right: 8px;
    z-index: 700;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(22, 22, 26, 0.7);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--fg-dim);
    cursor: pointer;
    padding: 0;
    backdrop-filter: blur(4px);
  }
  .gear:hover {
    color: var(--fg);
    border-color: var(--border-active);
    background: var(--bg-elev);
  }
</style>
