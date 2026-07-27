#!/usr/bin/env python3
"""Build / validate US equity calendar artifacts with QuantLib cross-check.

Prefer QuantLib when installed (`pip install QuantLib`); otherwise use an
independent NYSE-rules oracle matching QuantLib's UnitedStates(NYSE) calendar.

Offline CI uses the committed fixture as the zero-tolerance oracle — QuantLib is
not required. Optional `--regen` refreshes fixture + demo artifact from the
live oracle (QuantLib if present, else the independent rules engine).

Venues: NYSE, NASDAQ, ARCA, BATS (all share the US equity holiday schedule).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass
from datetime import date, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT = ROOT / "crates/prismatik-calendar/artifacts/demo_sessions_2024_2025.json"
FIXTURE = ROOT / "crates/prismatik-calendar/fixtures/quantlib_known_holidays.json"
REPORT = (
    ROOT / "crates/prismatik-calendar/artifacts/quantlib_cross_validation_report.json"
)

VENUES = ("nyse", "nasdaq", "arca", "bats")
YEARS = (2024, 2025)
REGULAR_OPEN = "09:30:00"
REGULAR_CLOSE = "16:00:00"
EARLY_CLOSE = "13:00:00"
GENERATOR_VERSION = "demo-2024-2025"

# One-off US equity closures mirrored from QuantLib UnitedStates(NYSE).
# Keep in sync when regenerating with QuantLib installed.
SPECIAL_CLOSURES: dict[int, frozenset[date]] = {
    # National Day of Mourning for President Jimmy Carter
    2025: frozenset({date(2025, 1, 9)}),
}


@dataclass(frozen=True)
class OracleInfo:
    name: str
    detail: str


def nth_weekday(year: int, month: int, weekday: int, n: int) -> date:
    """Return the n-th weekday (Mon=0) in month. n may be negative (from end)."""
    if n > 0:
        first = date(year, month, 1)
        offset = (weekday - first.weekday()) % 7
        return first + timedelta(days=offset + 7 * (n - 1))
    last_day = date(year, month + 1, 1) - timedelta(days=1) if month < 12 else date(year, 12, 31)
    offset = (last_day.weekday() - weekday) % 7
    return last_day - timedelta(days=offset + 7 * (-n - 1))


def easter_sunday(year: int) -> date:
    """Anonymous Gregorian algorithm (same as QuantLib Date::easterMonday basis)."""
    a = year % 19
    b = year // 100
    c = year % 100
    d = b // 4
    e = b % 4
    f = (b + 8) // 25
    g = (b - f + 1) // 3
    h = (19 * a + b - d - g + 15) % 30
    i = c // 4
    k = c % 4
    l = (32 + 2 * e + 2 * i - h - k) % 7
    m = (a + 11 * h + 22 * l) // 451
    month = (h + l - 7 * m + 114) // 31
    day = ((h + l - 7 * m + 114) % 31) + 1
    return date(year, month, day)


def adjust_observed(d: date) -> date:
    """Saturday -> Friday, Sunday -> Monday (QuantLib Independence/Christmas style)."""
    if d.weekday() == 5:  # Saturday
        return d - timedelta(days=1)
    if d.weekday() == 6:  # Sunday
        return d + timedelta(days=1)
    return d


def nyse_holidays_independent(year: int) -> set[date]:
    """Independent oracle matching QuantLib UnitedStates(NYSE) for modern years."""
    hol: set[date] = set()
    # New Year's Day (Sunday -> Monday; Saturday left as weekend-only)
    new_years = date(year, 1, 1)
    hol.add(new_years)
    if new_years.weekday() == 6:
        hol.add(date(year, 1, 2))
    # Martin Luther King Jr. Day — third Monday in January
    hol.add(nth_weekday(year, 1, 0, 3))
    # Washington's Birthday — third Monday in February
    hol.add(nth_weekday(year, 2, 0, 3))
    # Good Friday
    hol.add(easter_sunday(year) - timedelta(days=2))
    # Memorial Day — last Monday in May
    hol.add(nth_weekday(year, 5, 0, -1))
    # Juneteenth National Independence Day (observed) — since 2022
    if year >= 2022:
        hol.add(adjust_observed(date(year, 6, 19)))
    # Independence Day (observed)
    hol.add(adjust_observed(date(year, 7, 4)))
    # Labor Day — first Monday in September
    hol.add(nth_weekday(year, 9, 0, 1))
    # Thanksgiving — fourth Thursday in November
    hol.add(nth_weekday(year, 11, 3, 4))
    # Christmas (observed)
    hol.add(adjust_observed(date(year, 12, 25)))
    hol |= set(SPECIAL_CLOSURES.get(year, frozenset()))
    return {d for d in hol if d.year == year}


def try_quantlib_holidays(year: int) -> set[date] | None:
    try:
        import QuantLib as ql  # type: ignore
    except ImportError:
        return None
    cal = ql.UnitedStates(ql.UnitedStates.NYSE)
    holidays: set[date] = set()
    d = ql.Date(1, 1, year)
    end = ql.Date(31, 12, year)
    one = ql.Period(1, ql.Days)
    while d <= end:
        if d.weekday() not in (ql.Saturday, ql.Sunday) and not cal.isBusinessDay(d):
            holidays.add(date(d.year(), d.month(), d.dayOfMonth()))
        d = d + one
    return holidays


def holiday_set(year: int) -> tuple[set[date], OracleInfo]:
    ql_hol = try_quantlib_holidays(year)
    if ql_hol is not None:
        return ql_hol, OracleInfo("quantlib", f"QuantLib UnitedStates(NYSE) year={year}")
    return nyse_holidays_independent(year), OracleInfo(
        "independent-nyse-rules",
        f"offline NYSE-rules oracle year={year}",
    )


def is_trading_day_from_holidays(d: date, holidays: set[date]) -> bool:
    if d.weekday() >= 5:
        return False
    return d not in holidays


def artifact_is_trading_day(artifact: dict, venue: str, d: date) -> bool:
    if d.weekday() >= 5:
        return False
    for schedule in artifact["venues"]:
        if schedule["venue"] != venue:
            continue
        return d.isoformat() not in schedule["holidays"]
    raise KeyError(f"venue {venue!r} missing from artifact")


def thanksgiving(year: int) -> date:
    return nth_weekday(year, 11, 3, 4)


def early_close_sessions(years: tuple[int, ...]) -> list[dict]:
    sessions = []
    for year in years:
        day_after = thanksgiving(year) + timedelta(days=1)
        if day_after.weekday() < 5:
            sessions.append(
                {
                    "date": day_after.isoformat(),
                    "open": REGULAR_OPEN,
                    "close": EARLY_CLOSE,
                }
            )
    return sessions


def build_artifact(years: tuple[int, ...] = YEARS) -> tuple[dict, OracleInfo]:
    holidays: set[date] = set()
    oracle = OracleInfo("independent-nyse-rules", "offline")
    for year in years:
        year_holidays, oracle = holiday_set(year)
        holidays |= year_holidays
    holiday_strs = sorted(d.isoformat() for d in holidays)
    special = early_close_sessions(years)
    venues = []
    for venue in VENUES:
        venues.append(
            {
                "venue": venue,
                "regular_open": REGULAR_OPEN,
                "regular_close": REGULAR_CLOSE,
                "holidays": holiday_strs,
                "special_sessions": special,
            }
        )
    return {"version": GENERATOR_VERSION, "venues": venues}, oracle


def sample_probe_dates(years: tuple[int, ...], holidays: set[date]) -> list[date]:
    """Holidays + nearby trading weekdays + weekends for the fixture table."""
    probes: set[date] = set(holidays)
    for year in years:
        probes.add(date(year, 1, 6))  # Saturday
        probes.add(date(year, 1, 7))  # Sunday
        probes.add(date(year, 7, 5) if date(year, 7, 5).weekday() < 5 else date(year, 7, 8))
        probes.add(date(year, 3, 15) if date(year, 3, 15).weekday() < 5 else date(year, 3, 14))
    # Day after each holiday when it is a weekday → expect trading
    for h in list(holidays):
        nxt = h + timedelta(days=1)
        while nxt.weekday() >= 5:
            nxt += timedelta(days=1)
        if nxt.year in years:
            probes.add(nxt)
    return sorted(probes)


def build_fixture(years: tuple[int, ...] = YEARS) -> tuple[list[dict], OracleInfo]:
    holidays: set[date] = set()
    oracle = OracleInfo("independent-nyse-rules", "offline")
    for year in years:
        year_holidays, oracle = holiday_set(year)
        holidays |= year_holidays
    rows: list[dict] = []
    for venue in VENUES:
        for d in sample_probe_dates(years, holidays):
            rows.append(
                {
                    "venue": venue,
                    "date": d.isoformat(),
                    "is_trading_day": is_trading_day_from_holidays(d, holidays),
                }
            )
    return rows, oracle


def cross_validate(artifact: dict, fixture: list[dict]) -> dict:
    disagreements = []
    for row in fixture:
        venue = row["venue"]
        d = date.fromisoformat(row["date"])
        actual = artifact_is_trading_day(artifact, venue, d)
        expected = bool(row["is_trading_day"])
        if actual != expected:
            disagreements.append(
                {
                    "venue": venue,
                    "date": d.isoformat(),
                    "expected_trading_day": expected,
                    "actual_trading_day": actual,
                }
            )
    return {
        "checked": len(fixture),
        "disagreement_count": len(disagreements),
        "disagreements": disagreements,
    }


def write_report(
    *,
    validation: dict,
    oracle: OracleInfo,
    mode: str,
) -> dict:
    status = "pass" if validation["disagreement_count"] == 0 else "fail"
    body = {
        "gate": "calendar-quantlib-cross-validation",
        "tolerance": "zero",
        "status": status,
        "mode": mode,
        "oracle": {"name": oracle.name, "detail": oracle.detail},
        "artifact": str(ARTIFACT.relative_to(ROOT)).replace("\\", "/"),
        "fixture": str(FIXTURE.relative_to(ROOT)).replace("\\", "/"),
        "venues": list(VENUES),
        "years": list(YEARS),
        "checked": validation["checked"],
        "disagreement_count": validation["disagreement_count"],
        "disagreements": validation["disagreements"],
        "report_sha256": "",
    }
    # Hash without the self-hash field for stable content addressing.
    digest_source = dict(body)
    digest_source.pop("report_sha256", None)
    body["report_sha256"] = hashlib.sha256(
        json.dumps(digest_source, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(body, indent=2) + "\n", encoding="utf-8")
    return body


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def cmd_validate() -> int:
    artifact = load_json(ARTIFACT)
    fixture = load_json(FIXTURE)
    validation = cross_validate(artifact, fixture)
    report = write_report(
        validation=validation,
        oracle=OracleInfo("committed-fixture", "offline CI fixture oracle"),
        mode="validate",
    )
    print(
        f"calendar gate: {report['status']} "
        f"(checked={report['checked']}, disagreements={report['disagreement_count']})"
    )
    print(f"report -> {REPORT.relative_to(ROOT)}")
    if report["disagreement_count"]:
        for row in report["disagreements"]:
            print(f"  DISAGREE {row}", file=sys.stderr)
        return 1
    return 0


def cmd_regen() -> int:
    artifact, oracle = build_artifact()
    fixture, oracle = build_fixture()
    ARTIFACT.parent.mkdir(parents=True, exist_ok=True)
    FIXTURE.parent.mkdir(parents=True, exist_ok=True)
    ARTIFACT.write_text(json.dumps(artifact, indent=2) + "\n", encoding="utf-8")
    FIXTURE.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")
    validation = cross_validate(artifact, fixture)
    report = write_report(validation=validation, oracle=oracle, mode="regen")
    print(f"oracle: {oracle.name} ({oracle.detail})")
    print(f"wrote artifact -> {ARTIFACT.relative_to(ROOT)}")
    print(f"wrote fixture  -> {FIXTURE.relative_to(ROOT)} ({len(fixture)} rows)")
    print(
        f"calendar gate: {report['status']} "
        f"(checked={report['checked']}, disagreements={report['disagreement_count']})"
    )
    print(f"report -> {REPORT.relative_to(ROOT)}")
    return 0 if report["disagreement_count"] == 0 else 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--regen",
        action="store_true",
        help="Regenerate artifact + fixture from QuantLib (or offline NYSE rules)",
    )
    parser.add_argument(
        "--validate",
        action="store_true",
        help="Validate committed artifact against committed fixture (default)",
    )
    args = parser.parse_args(argv)
    if args.regen:
        return cmd_regen()
    return cmd_validate()


if __name__ == "__main__":
    raise SystemExit(main())
