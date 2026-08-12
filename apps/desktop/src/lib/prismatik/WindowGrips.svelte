<script lang="ts">
  /**
   * Edge/corner resize affordances for the decorationless window.
   *
   * With `decorations: false` the compositor no longer supplies a resize
   * border, so the frontend has to hand `startResizeDragging` the direction
   * itself. Grips are inert (and hidden) outside the Tauri runtime.
   *
   * Never rendered on macOS: `drag_resize_window` is unimplemented there, and
   * a grip that cannot resize would still capture the pointer and block the
   * native edge resize the window already has. See `platform.svelte.ts`.
   */
  import { platform } from './platform.svelte';

  type Direction =
    | 'North'
    | 'NorthEast'
    | 'East'
    | 'SouthEast'
    | 'South'
    | 'SouthWest'
    | 'West'
    | 'NorthWest';

  const EDGES: Direction[] = ['North', 'East', 'South', 'West'];
  const CORNERS: Direction[] = ['NorthWest', 'NorthEast', 'SouthEast', 'SouthWest'];

  async function beginResize(event: PointerEvent, direction: Direction) {
    if (event.button !== 0) return;
    event.preventDefault();
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().startResizeDragging(direction as never);
    } catch {
      /* resize can race with focus loss or a maximized window; non-fatal */
    }
  }
</script>

{#if platform.usesFrontendResizeGrips}
  <div class="pk-grips" aria-hidden="true">
    {#each [...EDGES, ...CORNERS] as direction (direction)}
      <span
        class="pk-grip pk-grip-{direction.toLowerCase()}"
        role="presentation"
        onpointerdown={(event) => void beginResize(event, direction)}
      ></span>
    {/each}
  </div>
{/if}

<style>
  .pk-grips {
    position: fixed;
    inset: 0;
    z-index: 9000;
    pointer-events: none;
  }

  .pk-grip {
    position: absolute;
    pointer-events: auto;
  }

  /* Edges: 4px hit strip, inset by the corner size so corners win. */
  .pk-grip-north,
  .pk-grip-south {
    left: 10px;
    right: 10px;
    height: 4px;
    cursor: ns-resize;
  }
  .pk-grip-north {
    top: 0;
  }
  .pk-grip-south {
    bottom: 0;
  }

  .pk-grip-west,
  .pk-grip-east {
    top: 10px;
    bottom: 10px;
    width: 4px;
    cursor: ew-resize;
  }
  .pk-grip-west {
    left: 0;
  }
  .pk-grip-east {
    right: 0;
  }

  /* Corners: 10px square. */
  .pk-grip-northwest,
  .pk-grip-northeast,
  .pk-grip-southeast,
  .pk-grip-southwest {
    width: 10px;
    height: 10px;
  }
  .pk-grip-northwest {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }
  .pk-grip-northeast {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }
  .pk-grip-southeast {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
  .pk-grip-southwest {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }
</style>
