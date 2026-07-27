# PRISMATIK public docs site (scaffold)

**Status:** floor scaffold — `mkdocs.yml` + `docs/index.md` present; no full MkDocs/CI site build required under turbo.

## Intent

Host user-facing guides and wave gate summaries as a static public site. Prefer MkDocs Material (or equivalent static generator) when author-ops wires publish.

## Layout

```text
DOCS/site/
  README.md          ← this file
  mkdocs.yml         ← nav + theme scaffold
  docs/
    index.md         ← published home (relative links into ../user/, ../gates/, ../waves/)
```

## Index (links into the repo today)

See [`docs/index.md`](docs/index.md) and the outline in [`mkdocs.yml`](mkdocs.yml).

### User guides

- [MVP User Guide](../user/MVP_User_Guide.md)
- [Execution runbooks](../user/execution-runbooks.md)

### Wave gates

- [Wave 0](../gates/wave-0.md)
- [Wave 1](../gates/wave-1.md)
- [Wave 2](../gates/wave-2.md)
- [Wave 3](../gates/wave-3.md)
- [Wave 4](../gates/wave-4.md)
- [Wave 5](../gates/wave-5.md)
- [Wave 6](../gates/wave-6.md)
- [Wave 7](../gates/wave-7.md)

### Related

- [Workbook](../workbook.md)
- [Turbo gate policy](../waves/TURBO_GATE_POLICY.md)

## Author-ops residual

Publish pipeline (Pages / Cloudflare / etc.), theme polish, and mirrored copies under `docs/` remain **author-ops** (`P9-OD-02`).
