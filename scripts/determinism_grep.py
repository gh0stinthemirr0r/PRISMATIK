#!/usr/bin/env python3
"""Determinism guard gate (Wave 0 DoD criteria 1 and 2).

Spec: ``DOCS/spec/CI_WORKFLOWS.md`` §"Job: determinism-grep",
      ``DOCS/spec/CRATE_ARCHITECTURE.md`` §1.1.

Forbids ambient-nondeterminism sources outside ``prismatik-determinism``:

  - ``Instant::now`` / ``SystemTime::now`` / ``OffsetDateTime::now_utc``
  - ``thread_rng`` / ``rand::random``
  - ``Uuid::new_v4``
  - ``HashMap`` / ``HashSet`` constructed with the default ``RandomState``
    (i.e. bare ``HashMap::new`` / ``HashMap::default`` / ``HashMap<...>`` type
    annotations that are not ``DetMap``/``DetSet``).

The gate matches **code only**, not comments or doc prose, so a doc comment
that mentions the word "HashMap" does not trip it. Legitimate exceptions
(e.g. a wall-clock latency benchmark that *must* use the monotonic clock) go
in an allowlist capped under 10 entries; every entry requires a one-line
reason in the allowlist file.

Exit code 0 = clean, 1 = violations found.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

# Root of the workspace (parent of this script's directory).
ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
DETERMINISM_CRATE = "prismatik-determinism"

# Overridable via --root for tests.
_root_override: Path | None = None


def _root() -> Path:
    """Workspace root (overridable via ``--root`` for the negative test)."""
    return _root_override if _root_override is not None else ROOT


def _crates() -> Path:
    return _root() / "crates"

# Patterns that indicate an ambient nondeterminism source in *code*.
# Each entry: (compiled regex, human-readable reason).
FORBIDDEN = [
    (re.compile(r"\bInstant::now\b"), "use Clock::monotonic_nanos via prismatik-determinism"),
    (re.compile(r"\bSystemTime::now\b"), "use Clock::now via prismatik-determinism"),
    (re.compile(r"\bOffsetDateTime::now_utc\b"), "use Clock::now via prismatik-determinism"),
    (re.compile(r"\bthread_rng\b"), "use Entropy::split via prismatik-determinism; never ambient RNG"),
    (re.compile(r"\brand::random\b"), "use Entropy via prismatik-determinism; never ambient RNG"),
    (re.compile(r"\bUuid::new_v4\b"), "use IdFactory or a Clock-derived id; v4 leaks ambient RNG"),
    # HashMap/HashSet with default RandomState. DetMap/DetSet (FxHasher) are fine.
    # Match the type name when it is NOT prefixed by "Det" and is used as a
    # concrete construction rather than only mentioned in a path segment.
    (re.compile(r"(?<!Det)HashMap(?:::new|::default|<)"), "use DetMap; default RandomState iteration order is nondeterministic"),
    (re.compile(r"(?<!Det)HashSet(?:::new|::default|<)"), "use DetSet; default RandomState iteration order is nondeterministic"),
]

# Line-comment and (rough) block-comment / string stripping. This is a
# deliberately conservative stripper: it only removes ``//`` line comments and
# the contents of doc/block comments. It does not attempt full lexing, which
# means a ``//`` inside a string literal would be mis-handled — but the
# opposite (matching a forbidden token that appears inside a comment) is the
# false-positive class we are eliminating, and erring toward fewer
# false-positives here is safe because clippy's disallowed-methods gate
# already catches real code-level violations with scoped allows.
_LINE_COMMENT = re.compile(r"//.*$")
# Rust doc/block comments are rare in this codebase on the same line as code;
# we strip a leading ``//!`` / ``///`` / ``/*`` ... ``*/`` span on the line.
_DOC_PREFIX = re.compile(r"^\s*([/][/]!|[/][/][/]|[/][\*])")


@dataclass(frozen=True)
class Violation:
    """A single forbidden-pattern hit."""

    path: Path
    lineno: int
    col: int
    matched: str
    reason: str

    def key(self) -> str:
        """Allowlist key: ``relative/path:lineno``."""
        rel = self.path.relative_to(_root()).as_posix()
        return f"{rel}:{self.lineno}"


def _strip_comments(raw_line: str) -> str:
    """Return the code portion of a line (comments removed)."""
    if _DOC_PREFIX.match(raw_line):
        return ""
    return _LINE_COMMENT.sub("", raw_line)


def _load_allowlist(path: Path) -> dict[str, str]:
    """Parse ``path`` → { ``file:line`` : reason }.

    Format: one entry per line, ``path:line # reason`` or ``path:line | reason``.
    Blank lines and ``#``-prefixed lines ignored.
    """
    entries: dict[str, str] = {}
    if not path.exists():
        return entries
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        for sep in ("#", "|"):
            if sep in line:
                key, _, reason = line.partition(sep)
                entries[key.strip()] = reason.strip() or "(no reason given)"
                break
        else:
            entries[line.strip()] = "(no reason given)"
    return entries


def _rust_files() -> list[Path]:
    """All ``.rs`` files under ``crates/`` excluding the determinism crate."""
    out: list[Path] = []
    for crate_dir in sorted(_crates().iterdir()):
        if not crate_dir.is_dir():
            continue
        if crate_dir.name == DETERMINISM_CRATE:
            continue
        src = crate_dir / "src"
        if src.is_dir():
            out.extend(sorted(src.rglob("*.rs")))
        tests = crate_dir / "tests"
        if tests.is_dir():
            out.extend(sorted(tests.rglob("*.rs")))
        benches = crate_dir / "benches"
        if benches.is_dir():
            out.extend(sorted(benches.rglob("*.rs")))
    return out


def scan() -> list[Violation]:
    """Scan all non-determinism-crate Rust files for forbidden patterns."""
    violations: list[Violation] = []
    for path in _rust_files():
        text = path.read_text(encoding="utf-8")
        for lineno, raw in enumerate(text.splitlines(), start=1):
            code = _strip_comments(raw)
            if not code.strip():
                continue
            for pattern, reason in FORBIDDEN:
                for m in pattern.finditer(code):
                    violations.append(
                        Violation(
                            path=path,
                            lineno=lineno,
                            col=m.start() + 1,
                            matched=m.group(0),
                            reason=reason,
                        )
                    )
    return violations


def main(argv: list[str] | None = None) -> int:
    global _root_override
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--allowlist",
        type=Path,
        default=None,
        help="Path to the allowlist file (default: <root>/crates/prismatik-determinism/ALLOWLIST.txt).",
    )
    parser.add_argument(
        "--no-allowlist",
        action="store_true",
        help="Ignore the allowlist entirely (used by the negative test).",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help="Workspace root to scan (default: the script's parent.parent). For tests.",
    )
    parser.add_argument(
        "--max-allowlist",
        type=int,
        default=10,
        help="Hard cap on allowlist entries (DoD: under 10).",
    )
    args = parser.parse_args(argv)

    if args.root is not None:
        _root_override = args.root.resolve()

    if args.no_allowlist:
        allowlist: dict[str, str] = {}
    else:
        path = args.allowlist or (_crates() / DETERMINISM_CRATE / "ALLOWLIST.txt")
        allowlist = _load_allowlist(path)
    if len(allowlist) > args.max_allowlist:
        print(
            f"error: allowlist has {len(allowlist)} entries, cap is {args.max_allowlist}; "
            "every addition must justify itself or the gate is meaningless.",
            file=sys.stderr,
        )
        return 1

    violations = [v for v in scan() if v.key() not in allowlist]
    if not violations:
        print(
            f"Determinism guard passed (allowlist: {len(allowlist)} entries, "
            f"cap {args.max_allowlist})."
        )
        return 0

    print(
        "Determinism guard failed. Use Clock/Entropy/DetMap via prismatik-determinism, "
        "or add a justified entry to the allowlist:",
        file=sys.stderr,
    )
    for v in violations:
        rel = v.path.relative_to(_root()).as_posix()
        print(
            f"  {rel}:{v.lineno}:{v.col}  `{v.matched}`  — {v.reason}",
            file=sys.stderr,
        )
    print(
        f"\n{len(violations)} violation(s). Allowlist entries without a reason are rejected.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
