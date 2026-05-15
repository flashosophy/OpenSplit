/**
 * Per-pane handle that Terminal.svelte registers on mount and unregisters on
 * destroy. App.svelte uses these handles to act on the focused pane (e.g.
 * read its current xterm selection for copy, or push pasted text into it).
 *
 * Kept deliberately minimal — only the operations we actually need. Adding
 * more methods is cheap; growing the surface without need is not.
 */
export interface TerminalHandle {
  /** xterm selected text, or empty string if nothing selected. */
  getSelection(): string;
  /** Clear any current xterm selection. */
  clearSelection(): void;
  /** Push text into the PTY as if the user typed it. Honors bracketed paste. */
  paste(text: string): void;
  /** Move keyboard focus into this terminal. */
  focus(): void;
}

const handles = new Map<string, TerminalHandle>();

export function registerTerminal(paneId: string, handle: TerminalHandle): void {
  handles.set(paneId, handle);
}

export function unregisterTerminal(paneId: string): void {
  handles.delete(paneId);
}

export function getTerminal(paneId: string): TerminalHandle | undefined {
  return handles.get(paneId);
}
