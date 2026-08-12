#!/usr/bin/env python3
"""Fail when production manifests depend on the local reference corpus."""

from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_NAMES = {"Cargo.toml", "package.json", "pnpm-workspace.yaml"}
SKIP_PARTS = {"references", "node_modules", "target", ".git"}
NEEDLES = ("references/", "references\\\\")


def main() -> int:
    violations: list[str] = []
    for path in ROOT.rglob("*"):
        if not path.is_file() or path.name not in MANIFEST_NAMES:
            continue
        if SKIP_PARTS.intersection(path.relative_to(ROOT).parts):
            continue
        text = path.read_text(encoding="utf-8")
        for line_number, line in enumerate(text.splitlines(), 1):
            if any(needle in line for needle in NEEDLES):
                violations.append(f"{path.relative_to(ROOT)}:{line_number}: {line.strip()}")

    if violations:
        print("Reference-boundary violation: production manifests must not depend on references/.")
        print("\n".join(violations))
        return 1

    print("Reference boundary passed: production manifests are native-only.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
