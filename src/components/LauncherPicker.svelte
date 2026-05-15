<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { DetectedTool } from "../lib/ipc";
  import { detectTools, setDefaultProfile } from "../lib/ipc";

  interface Props {
    detected: DetectedTool[];
    /** Called when the user picks a tool. */
    onPick: (tool: DetectedTool, setAsDefault: boolean) => void;
  }

  let { detected, onPick }: Props = $props();

  // Local copy that we can mutate on refresh. We intentionally re-init from
  // the prop on mount (rather than $derived) because once the picker shows
  // the parent doesn't push updates.
  let tools = $state<DetectedTool[]>([]);
  $effect(() => {
    if (tools.length === 0 && detected.length > 0) {
      tools = detected;
    }
  });
  let focusIndex = $state(0);
  let setAsDefault = $state(false);
  let refreshing = $state(false);

  function isLaunchable(t: DetectedTool): boolean {
    // The "shell" entry is always launchable; otherwise need a resolved path.
    return t.name === "shell" || t.path !== null;
  }

  /** Sorted view: launchable first, AI category before terminal/custom. */
  let launchable = $derived(tools.filter(isLaunchable));

  function moveFocus(delta: number) {
    if (launchable.length === 0) return;
    focusIndex =
      (focusIndex + delta + launchable.length) % launchable.length;
  }

  function pickByIndex(i: number) {
    const t = launchable[i];
    if (!t) return;
    onPick(t, setAsDefault);
  }

  async function refresh() {
    refreshing = true;
    try {
      tools = await detectTools();
      focusIndex = Math.min(focusIndex, Math.max(0, launchable.length - 1));
    } catch (e) {
      console.error("[opensplit] refresh failed", e);
    } finally {
      refreshing = false;
    }
  }

  function onKeydown(ev: KeyboardEvent) {
    // Don't trap shortcuts that App.svelte cares about.
    if (ev.ctrlKey || ev.metaKey || ev.altKey) return;

    if (ev.key === "ArrowDown" || ev.key === "ArrowRight") {
      ev.preventDefault();
      moveFocus(1);
    } else if (ev.key === "ArrowUp" || ev.key === "ArrowLeft") {
      ev.preventDefault();
      moveFocus(-1);
    } else if (ev.key === "Enter") {
      ev.preventDefault();
      pickByIndex(focusIndex);
    } else if (/^[1-9]$/.test(ev.key)) {
      const idx = parseInt(ev.key, 10) - 1;
      if (idx < launchable.length) {
        ev.preventDefault();
        pickByIndex(idx);
      }
    } else if (ev.key === "d" || ev.key === "D") {
      ev.preventDefault();
      setAsDefault = !setAsDefault;
    } else if (ev.key === "r" || ev.key === "R") {
      ev.preventDefault();
      void refresh();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeydown);
  });
</script>

<div class="picker">
  <div class="picker-inner">
    <h1 class="title">OpenSplit</h1>
    <p class="subtitle">Pick a tool to launch.</p>

    {#if launchable.length === 0}
      <div class="empty">
        <p>No tools detected on this system.</p>
        <p>
          Install one of: <code>opencode</code>, <code>codex</code>,
          <code>claude</code>, <code>aider</code>, etc., or click Refresh.
        </p>
      </div>
    {/if}

    <div class="grid" aria-label="Launchable tools">
      {#each launchable as tool, i (tool.name)}
        <button
          type="button"
          class="tile"
          class:focused={i === focusIndex}
          class:default-marker={tool.has_profile && tool.icon !== "terminal"}
          onclick={() => pickByIndex(i)}
          onmouseenter={() => (focusIndex = i)}
        >
          <span class="tile-icon" aria-hidden="true">
            {#if tool.icon === "ai"}
              <svg viewBox="0 0 24 24" width="28" height="28">
                <path
                  d="M12 2.5l2.2 5.1 5.5.5-4.2 3.7 1.3 5.4L12 14.4 7.2 17.2l1.3-5.4L4.3 8.1l5.5-.5z"
                  fill="none" stroke="currentColor" stroke-width="1.4"
                  stroke-linejoin="round"
                />
              </svg>
            {:else if tool.icon === "terminal"}
              <svg viewBox="0 0 24 24" width="28" height="28">
                <rect x="2.5" y="4.5" width="19" height="15" rx="2"
                  fill="none" stroke="currentColor" stroke-width="1.4"/>
                <polyline points="6,9 10,12 6,15" fill="none"
                  stroke="currentColor" stroke-width="1.4"
                  stroke-linecap="round" stroke-linejoin="round"/>
                <line x1="12" y1="15" x2="17" y2="15"
                  stroke="currentColor" stroke-width="1.4"
                  stroke-linecap="round"/>
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" width="28" height="28">
                <circle cx="12" cy="12" r="9" fill="none"
                  stroke="currentColor" stroke-width="1.4"/>
                <path d="M8 12l3 3 5-6" fill="none"
                  stroke="currentColor" stroke-width="1.4"
                  stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            {/if}
          </span>
          <span class="tile-label">{tool.label}</span>
          <span class="tile-desc">{tool.description}</span>
          {#if i < 9}
            <span class="tile-key">{i + 1}</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="footer">
      <label class="checkbox">
        <input type="checkbox" bind:checked={setAsDefault} />
        <span>Remember this as my default <kbd>D</kbd></span>
      </label>
      <div class="footer-spacer"></div>
      <button
        type="button"
        class="ghost"
        onclick={refresh}
        disabled={refreshing}
      >
        {refreshing ? "Scanning…" : "Refresh"} <kbd>R</kbd>
      </button>
    </div>

    <p class="hint">
      <kbd>↑</kbd><kbd>↓</kbd> navigate · <kbd>Enter</kbd> select ·
      <kbd>1</kbd>–<kbd>9</kbd> quick pick
    </p>
  </div>
</div>

<style>
  .picker {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
    overflow: auto;
    padding: 24px;
  }
  .picker-inner {
    width: 100%;
    max-width: 760px;
  }
  .title {
    font-size: 28px;
    font-weight: 600;
    margin: 0 0 4px;
    color: var(--fg);
    text-align: center;
  }
  .subtitle {
    margin: 0 0 28px;
    color: var(--fg-dim);
    text-align: center;
    font-size: 14px;
  }
  .empty {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    margin: 0 0 20px;
    color: var(--fg-dim);
    text-align: center;
  }
  .empty code {
    background: var(--bg);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--fg);
    font-size: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
    margin-bottom: 20px;
  }
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 14px 16px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 8px;
    text-align: left;
    cursor: pointer;
    transition: background 80ms ease, border-color 80ms ease, transform 80ms ease;
    color: var(--fg);
  }
  .tile:hover {
    background: var(--menu-hover);
  }
  .tile.focused {
    border-color: var(--border-active);
    background: var(--menu-hover);
  }
  .tile:active {
    transform: scale(0.99);
  }
  .tile-icon {
    color: var(--accent);
    line-height: 0;
  }
  .tile-label {
    font-size: 15px;
    font-weight: 600;
  }
  .tile-desc {
    font-size: 12px;
    color: var(--fg-dim);
    line-height: 1.35;
  }
  .tile-key {
    position: absolute;
    top: 10px;
    right: 12px;
    font-size: 10px;
    color: var(--fg-dim);
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 1px 5px;
    border-radius: 3px;
    font-variant-numeric: tabular-nums;
  }
  .default-marker::after {
    content: "★";
    position: absolute;
    bottom: 10px;
    right: 12px;
    color: var(--accent);
    font-size: 12px;
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .footer-spacer {
    flex: 1;
  }
  .checkbox {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--fg-dim);
    font-size: 12px;
    cursor: pointer;
  }
  .checkbox input {
    accent-color: var(--accent);
  }
  .ghost {
    background: transparent;
    border: 1px solid var(--border);
  }
  .ghost:hover:not(:disabled) {
    border-color: var(--border-active);
    background: var(--menu-hover);
  }
  .ghost:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .hint {
    text-align: center;
    color: var(--fg-dim);
    font-size: 11px;
    margin: 4px 0 0;
  }
  kbd {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 4px;
    font-size: 10px;
    font-family: inherit;
    color: var(--fg);
  }
</style>
