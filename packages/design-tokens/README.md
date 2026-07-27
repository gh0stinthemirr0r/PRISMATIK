# @prismatik/design-tokens

Shared CSS variables for the PRISMATIK interface, maintained by Mythos Systems.

Import the theme once at the application entry point:

```ts
import "@prismatik/design-tokens/tokens.css";
```

`tokens.css` contains light defaults, dark-theme overrides via
`[data-theme="dark"]`, system dark-mode defaults, and reduced-motion overrides.

Run `node scripts/check-contrast.mjs` to validate the key body-text and surface
pairs against WCAG AA's 4.5:1 contrast requirement. Semantic color tokens
communicate state and must not be the sole indicator of meaning.
