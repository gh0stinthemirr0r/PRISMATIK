/**
 * Host-platform facts the chrome has to branch on.
 *
 * PRISMATIK draws its own title bar, and that is not the same job on every
 * desktop:
 *
 * - **Windows / Linux** run the window `decorations: false` and the frontend
 *   supplies minimise / maximise / close on the right, plus resize grips.
 * - **macOS** runs `decorations: true` with `titleBarStyle: "Overlay"` and
 *   `hiddenTitle` (see `tauri.macos.conf.json`), so the system keeps the
 *   traffic lights. Drawing our own controls there would duplicate them on the
 *   wrong side, and the grips are worse than useless: tao returns
 *   `NotSupported` for `drag_resize_window` on macOS, so a grip cannot resize —
 *   but its hit strip would still swallow the mousedown and block the native
 *   edge resize that a borderless resizable window otherwise provides.
 */

export type HostPlatform = 'macos' | 'windows' | 'linux';

const inTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

class PlatformStore {
  /** Null until resolved; treated as "not macOS" for layout purposes. */
  host = $state<HostPlatform | null>(null);
  resolved = $state(false);

  readonly isTauri = inTauri;

  /** Traffic lights are supplied by the system, so we must not draw controls. */
  usesSystemWindowControls = $derived(this.host === 'macos');

  /** Only platforms where `drag_resize_window` is implemented get grips. */
  usesFrontendResizeGrips = $derived(
    this.isTauri && this.resolved && this.host !== 'macos',
  );

  /** The modifier this platform actually labels its shortcuts with. */
  modifierLabel = $derived(this.host === 'macos' ? '⌘' : 'Ctrl');

  async resolve(): Promise<void> {
    if (this.resolved) return;
    if (!inTauri) {
      // Browser dev: no window chrome is drawn at all, so the value is unused.
      this.resolved = true;
      return;
    }
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      this.host = await invoke<HostPlatform>('host_platform');
    } catch {
      // Fall back to the conservative option: no grips, no custom controls.
      this.host = 'macos';
    } finally {
      this.resolved = true;
    }
  }
}

export const platform = new PlatformStore();
