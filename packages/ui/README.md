# @prismatik/ui

Svelte 5 UI primitives for PRISMATIK, maintained by Mythos Systems.

```svelte
<script lang="ts">
  import { Button, StaleDataMarker } from "@prismatik/ui";
</script>

<Button variant="primary">Run analysis</Button>
<StaleDataMarker eventTime={quote.asOf} maxAge={30_000} />
```

The package imports `@prismatik/design-tokens/tokens.css` at its entry point.
Components use Svelte 5 runes and retain visible status for evidence (I1),
coverage (I2), and stale data (I6). Import `@prismatik/ui/motion` for the
reduced-motion-aware motion class names.
