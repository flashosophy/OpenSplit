import {
  readText as tauriReadText,
  writeText as tauriWriteText,
} from "@tauri-apps/plugin-clipboard-manager";

/**
 * Thin wrapper over the Tauri clipboard plugin with a `navigator.clipboard`
 * fallback for the dev server case where the Tauri plugin might not be loaded
 * yet (e.g. inside a non-Tauri preview).
 *
 * Both functions swallow nothing — callers should `.catch()` and decide
 * whether to surface the error.
 */

export async function copyText(text: string): Promise<void> {
  if (!text) return;
  try {
    await tauriWriteText(text);
  } catch (e) {
    // Fallback for dev / non-Tauri contexts. Will fail in some browsers
    // without a recent user gesture, but at that point the dev console
    // error is informative enough.
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }
    throw e;
  }
}

export async function pasteText(): Promise<string> {
  try {
    const t = await tauriReadText();
    return t ?? "";
  } catch (e) {
    if (navigator.clipboard?.readText) {
      return await navigator.clipboard.readText();
    }
    throw e;
  }
}
