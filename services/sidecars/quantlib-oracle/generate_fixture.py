#!/usr/bin/env python3
"""Generate Black-Scholes conformance fixtures (QuantLib-compatible erf CDF).

Prefer QuantLib when installed (`pip install QuantLib`); otherwise use an
independent erf-based analytic engine matching the QuantLib closed form.
"""

from __future__ import annotations

import itertools
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT_500 = ROOT / "crates/prismatik-quant-kernel/tests/fixtures/quantlib_conformance_500.json"
OUT_20 = ROOT / "crates/prismatik-quant-kernel/tests/fixtures/black_scholes.json"


def pdf(x: float) -> float:
    return math.exp(-0.5 * x * x) / math.sqrt(2.0 * math.pi)


def cdf(x: float) -> float:
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def black_scholes(
    spot: float,
    strike: float,
    t: float,
    r: float,
    q: float,
    vol: float,
    kind: str,
) -> dict[str, float]:
    sqrt_t = math.sqrt(t)
    d1 = (math.log(spot / strike) + (r - q + 0.5 * vol * vol) * t) / (vol * sqrt_t)
    d2 = d1 - vol * sqrt_t
    spot_d = math.exp(-q * t)
    strike_d = math.exp(-r * t)
    dens = pdf(d1)
    gamma = spot_d * dens / (spot * vol * sqrt_t)
    vega = spot * spot_d * dens * sqrt_t
    diffusion = -(spot * spot_d * dens * vol) / (2.0 * sqrt_t)
    if kind == "Call":
        nd1, nd2 = cdf(d1), cdf(d2)
        price = spot * spot_d * nd1 - strike * strike_d * nd2
        delta = spot_d * nd1
        theta = diffusion - r * strike * strike_d * nd2 + q * spot * spot_d * nd1
    else:
        nmd1, nmd2 = cdf(-d1), cdf(-d2)
        price = strike * strike_d * nmd2 - spot * spot_d * nmd1
        delta = -spot_d * nmd1
        theta = diffusion + r * strike * strike_d * nmd2 - q * spot * spot_d * nmd1
    return {
        "price": price,
        "delta": delta,
        "gamma": gamma,
        "vega": vega,
        "theta": theta,
    }


def try_quantlib(
    spot: float,
    strike: float,
    t: float,
    r: float,
    q: float,
    vol: float,
    kind: str,
) -> dict[str, float] | None:
    try:
        import QuantLib as ql  # type: ignore
    except ImportError:
        return None
    today = ql.Date(1, 1, 2026)
    ql.Settings.instance().evaluationDate = today
    exercise = ql.EuropeanExercise(today + int(round(t * 365)))
    payoff = ql.PlainVanillaPayoff(
        ql.Option.Call if kind == "Call" else ql.Option.Put, strike
    )
    process = ql.BlackScholesMertonProcess(
        ql.QuoteHandle(ql.SimpleQuote(spot)),
        ql.YieldTermStructureHandle(ql.FlatForward(today, q, ql.Actual365Fixed())),
        ql.YieldTermStructureHandle(ql.FlatForward(today, r, ql.Actual365Fixed())),
        ql.BlackVolTermStructureHandle(
            ql.BlackConstantVol(today, ql.NullCalendar(), vol, ql.Actual365Fixed())
        ),
    )
    option = ql.VanillaOption(payoff, exercise)
    option.setPricingEngine(ql.AnalyticEuropeanEngine(process))
    return {
        "price": option.NPV(),
        "delta": option.delta(),
        "gamma": option.gamma(),
        "vega": option.vega(),
        "theta": option.theta(),
    }


def build_cases(limit: int = 500) -> list[dict]:
    spots = [50, 80, 95, 100, 105, 120, 150, 200, 250, 500]
    strikes = [40, 50, 55, 80, 95, 100, 105, 120, 175, 200, 220, 500]
    times = [0.01, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0]
    rates = [-0.01, 0.0, 0.01, 0.03, 0.05, 0.07]
    divs = [0.0, 0.01, 0.015, 0.02, 0.03]
    vols = [0.08, 0.1, 0.15, 0.18, 0.2, 0.22, 0.25, 0.3, 0.35, 0.4, 0.5, 0.9]
    kinds = ["Call", "Put"]
    cases: list[dict] = []
    seen: set[tuple] = set()
    for spot, strike, t, r, q, vol, kind in itertools.product(
        spots, strikes, times, rates, divs, vols, kinds
    ):
        if len(cases) >= limit:
            break
        key = (spot, strike, t, r, q, vol, kind)
        if key in seen:
            continue
        if abs(math.log(spot / strike)) > 1.2:
            continue
        expected = try_quantlib(spot, strike, t, r, q, vol, kind)
        if expected is None:
            expected = black_scholes(spot, strike, t, r, q, vol, kind)
        if not all(math.isfinite(v) for v in expected.values()):
            continue
        seen.add(key)
        cases.append(
            {
                "name": f"grid_{len(cases) + 1:03d}",
                "spot": spot,
                "strike": strike,
                "time_to_expiry": t,
                "risk_free_rate": r,
                "dividend_yield": q,
                "volatility": vol,
                "kind": kind,
                "expected": expected,
            }
        )
    if len(cases) < limit:
        raise SystemExit(f"only generated {len(cases)} cases, need {limit}")
    return cases


def main() -> None:
    cases = build_cases(500)
    OUT_500.parent.mkdir(parents=True, exist_ok=True)
    OUT_500.write_text(json.dumps(cases, separators=(",", ":")), encoding="utf-8")
    small = []
    for i, case in enumerate(cases[:20], start=1):
        row = dict(case)
        row["name"] = f"case_{i:02d}"
        small.append(row)
    OUT_20.write_text(json.dumps(small, indent=2), encoding="utf-8")
    print(f"wrote {len(cases)} cases -> {OUT_500}")
    print(f"wrote {len(small)} cases -> {OUT_20}")


if __name__ == "__main__":
    main()
