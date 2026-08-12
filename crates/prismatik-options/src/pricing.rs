//! Black–Scholes–Merton pricing, greeks, and implied volatility.
//!
//! Pure and deterministic: the same inputs always produce the same outputs,
//! with no clock, no randomness and no I/O. That matters here more than
//! elsewhere, because everything downstream — the implied-vol surface, skew,
//! term structure, implied correlation for dispersion — is built by iterating
//! this module thousands of times, and a solver that wandered would make every
//! one of those numbers unreproducible.
//!
//! **Where a number does not exist, this module says so.** A quote below
//! intrinsic value has no implied volatility — not a small one, none at all,
//! because no volatility makes Black–Scholes produce that price. Bad marks,
//! stale quotes and crossed books are ordinary in options data, and the honest
//! response is [`ImpliedVolError`] rather than a plausible-looking figure
//! that would then propagate into a surface and out into a signal.
//!
//! Conventions, stated once because the sign and scale of a greek is exactly
//! the kind of thing that silently corrupts a book:
//!
//! - Rates, yields, volatilities and time are all **annualised decimals**.
//!   5% is `0.05`, not `5.0`. One month is `1.0 / 12.0`.
//! - **Vega is per 1.0 of volatility**, not per vol point. Divide by 100 for
//!   the "per 1% move" convention if that is what a display wants.
//! - **Theta is per year**, not per day. Divide by 365 for a daily decay.
//! - Dividend yield `q` is continuous. Pass `0.0` for a non-payer, and for an
//!   equity index use the index yield.

use crate::types::OptionType;

/// Below this many years to expiry an option is treated as expiring now.
///
/// About four minutes. Inside that window `σ√T` underflows toward zero and
/// the greeks diverge, so the value is its intrinsic worth and the greeks are
/// the degenerate ones rather than something numerically enormous.
const MIN_TIME: f64 = 1e-5;

/// Volatilities below this are treated as zero.
const MIN_VOL: f64 = 1e-9;

/// Search bounds for the implied-volatility solve, as annualised decimals.
///
/// 1000% is far past anything a real listed option trades at; the bound
/// exists so a pathological quote terminates rather than iterating forever.
const IV_LOWER: f64 = 1e-6;
const IV_UPPER: f64 = 10.0;

/// Newton iterations before falling back to bisection.
const NEWTON_STEPS: usize = 32;

/// Bisection iterations. 128 halvings of [1e-6, 10] is far past f64 precision,
/// so this bound is a guard against a non-terminating loop, not a limit on
/// achievable accuracy.
const BISECTION_STEPS: usize = 128;

/// Price convergence tolerance for the solver, *relative* to the quote.
///
/// An absolute tolerance is wrong here. A far out-of-the-money option worth
/// 1e-8 is almost flat in volatility, so an absolute band of 1e-10 is
/// satisfied across a wide range of volatilities and the solver "converges"
/// to whichever one it happened to land on. Scaling to the quote keeps the
/// criterion meaningful at every price.
const PRICE_TOLERANCE: f64 = 1e-12;

/// Width of the volatility bracket at which bisection stops.
///
/// Bisection converges on the *volatility* rather than the price for the same
/// reason: the bracket width is a direct statement about how well the answer
/// is pinned down, whereas a price difference near zero says almost nothing
/// about it.
const VOL_TOLERANCE: f64 = 1e-12;

/// The inputs a single option needs to be priced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlackScholesInputs {
    /// Spot price of the underlying.
    pub spot: f64,
    /// Strike price.
    pub strike: f64,
    /// Time to expiry in years.
    pub time_to_expiry: f64,
    /// Continuously compounded risk-free rate, annualised decimal.
    pub rate: f64,
    /// Continuous dividend yield, annualised decimal.
    pub dividend_yield: f64,
    /// Annualised volatility, decimal.
    pub volatility: f64,
    /// Call or put.
    pub option_type: OptionType,
}

impl BlackScholesInputs {
    /// Whether every input is finite and in a range the model is defined on.
    ///
    /// Negative spot or strike, negative time or negative volatility are not
    /// edge cases to clamp — they are wrong, and pricing them would produce a
    /// number that looks like a price.
    pub fn is_well_formed(&self) -> bool {
        self.spot.is_finite()
            && self.spot > 0.0
            && self.strike.is_finite()
            && self.strike > 0.0
            && self.time_to_expiry.is_finite()
            && self.time_to_expiry >= 0.0
            && self.rate.is_finite()
            && self.dividend_yield.is_finite()
            && self.volatility.is_finite()
            && self.volatility >= 0.0
    }
}

/// A full risk snapshot for one contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FullGreeks {
    /// Theoretical price.
    pub price: f64,
    /// ∂price/∂spot.
    pub delta: f64,
    /// ∂²price/∂spot².
    pub gamma: f64,
    /// ∂price/∂volatility, per 1.0 of vol.
    pub vega: f64,
    /// ∂price/∂time, per year. Negative for a long option under normal rates.
    pub theta: f64,
    /// ∂price/∂rate, per 1.0 of rate.
    pub rho: f64,
}

/// Standard normal probability density.
fn norm_pdf(x: f64) -> f64 {
    const INV_SQRT_2PI: f64 = 0.398_942_280_401_432_7;
    INV_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Standard normal cumulative distribution.
///
/// Hart's rational approximation, in the arrangement given by West (2005).
/// Accurate to roughly double precision across the whole range, which a
/// series approximation such as Abramowitz–Stegun 7.1.26 is not — that one is
/// good to about 1e-7, and a 1e-7 error in `N(d2)` shows up as a visible kink
/// in a fitted volatility surface.
fn norm_cdf(x: f64) -> f64 {
    let a = x.abs();
    // Beyond 37 standard deviations the result is 0 or 1 to within f64.
    if a > 37.0 {
        return if x > 0.0 { 1.0 } else { 0.0 };
    }
    let e = (-a * a / 2.0).exp();
    let tail = if a < 7.071_067_811_865_47 {
        let numerator = (((((3.526_249_659_989_11e-2 * a + 0.700_383_064_443_688) * a
            + 6.373_962_203_531_65)
            * a
            + 33.912_866_078_383)
            * a
            + 112.079_291_497_871)
            * a
            + 221.213_596_169_931)
            * a
            + 220.206_867_912_376;
        let denominator = ((((((8.838_834_764_831_84e-2 * a + 1.755_667_163_182_64) * a
            + 16.064_177_579_207)
            * a
            + 86.780_732_202_946_1)
            * a
            + 296.564_248_779_674)
            * a
            + 637.333_633_378_831)
            * a
            + 793.826_512_519_948)
            * a
            + 440.413_735_824_752;
        e * numerator / denominator
    } else {
        // Continued-fraction tail for the far wings.
        let q = a + 1.0 / (a + 2.0 / (a + 3.0 / (a + 4.0 / (a + 0.65))));
        e / (q * 2.506_628_274_631)
    };
    if x > 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

/// What an option is worth at expiry, discounted for carry.
///
/// Used for the degenerate cases — expired, or zero volatility — where the
/// option is a deterministic claim rather than a distribution.
fn discounted_intrinsic(inputs: &BlackScholesInputs) -> f64 {
    let forward = inputs.spot * (-inputs.dividend_yield * inputs.time_to_expiry).exp();
    let discounted_strike = inputs.strike * (-inputs.rate * inputs.time_to_expiry).exp();
    match inputs.option_type {
        OptionType::Call => (forward - discounted_strike).max(0.0),
        OptionType::Put => (discounted_strike - forward).max(0.0),
    }
}

/// Theoretical price, or `None` when the inputs are not well formed.
pub fn price(inputs: &BlackScholesInputs) -> Option<f64> {
    greeks(inputs).map(|g| g.price)
}

/// Price and every first- and second-order greek.
///
/// Returns `None` rather than `NaN` when the inputs are outside the model's
/// domain: a caller that receives a number is entitled to assume it means
/// something.
pub fn greeks(inputs: &BlackScholesInputs) -> Option<FullGreeks> {
    if !inputs.is_well_formed() {
        return None;
    }

    // Expired, or no volatility at all: the payoff is determined, so the
    // option is worth its discounted intrinsic value. Delta is 1, -1 or 0 and
    // the convexity greeks are exactly zero — there is nothing left to be
    // uncertain about.
    if inputs.time_to_expiry < MIN_TIME || inputs.volatility < MIN_VOL {
        let value = discounted_intrinsic(inputs);
        let in_the_money = value > 0.0;
        let delta = match (inputs.option_type, in_the_money) {
            (OptionType::Call, true) => 1.0,
            (OptionType::Put, true) => -1.0,
            _ => 0.0,
        };
        return Some(FullGreeks {
            price: value,
            delta,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
        });
    }

    let BlackScholesInputs {
        spot,
        strike,
        time_to_expiry: t,
        rate: r,
        dividend_yield: q,
        volatility: sigma,
        option_type,
    } = *inputs;

    let sqrt_t = t.sqrt();
    let sigma_sqrt_t = sigma * sqrt_t;
    let d1 = ((spot / strike).ln() + (r - q + 0.5 * sigma * sigma) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;

    let discount_q = (-q * t).exp();
    let discount_r = (-r * t).exp();
    let pdf_d1 = norm_pdf(d1);

    // Gamma and vega are identical for calls and puts by put–call parity: the
    // difference between them is a forward, which is linear in spot and
    // insensitive to volatility.
    let gamma = discount_q * pdf_d1 / (spot * sigma_sqrt_t);
    let vega = spot * discount_q * pdf_d1 * sqrt_t;
    let shared_theta = -spot * discount_q * pdf_d1 * sigma / (2.0 * sqrt_t);

    let (price, delta, theta, rho) = match option_type {
        OptionType::Call => {
            let n_d1 = norm_cdf(d1);
            let n_d2 = norm_cdf(d2);
            (
                spot * discount_q * n_d1 - strike * discount_r * n_d2,
                discount_q * n_d1,
                shared_theta - r * strike * discount_r * n_d2 + q * spot * discount_q * n_d1,
                strike * t * discount_r * n_d2,
            )
        },
        OptionType::Put => {
            let n_neg_d1 = norm_cdf(-d1);
            let n_neg_d2 = norm_cdf(-d2);
            (
                strike * discount_r * n_neg_d2 - spot * discount_q * n_neg_d1,
                -discount_q * n_neg_d1,
                shared_theta + r * strike * discount_r * n_neg_d2
                    - q * spot * discount_q * n_neg_d1,
                -strike * t * discount_r * n_neg_d2,
            )
        },
    };

    Some(FullGreeks {
        price,
        delta,
        gamma,
        vega,
        theta,
        rho,
    })
}

/// Why a quoted price has no implied volatility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpliedVolError {
    /// Spot, strike, time or rate was not a usable number.
    MalformedInputs,
    /// The quote was zero, negative, or not finite.
    MalformedPrice,
    /// The option has expired, so there is no volatility to imply.
    Expired,
    /// The quote is below intrinsic value. No volatility produces it, and the
    /// quote itself is arbitrageable — usually a stale or crossed mark.
    BelowIntrinsic,
    /// The quote is at or above the maximum the model can produce (the
    /// discounted forward for a call, the discounted strike for a put).
    AboveMaximum,
    /// The quote implies a volatility outside the search bounds.
    OutsideSearchBounds,
    /// The quote carries no recoverable information about volatility.
    ///
    /// A deep in-the-money option whose time value has rounded away, or a far
    /// out-of-the-money one worth essentially nothing, prices identically
    /// across a wide band of volatilities. There is no unique answer, and
    /// returning the midpoint of that band would put a fabricated point into
    /// a surface — which is exactly how a bad skew number is born.
    Unidentifiable,
}

/// The no-arbitrage price range for these terms, at any volatility.
///
/// The lower bound is intrinsic value; the upper is the limit as volatility
/// goes to infinity — a call becomes the discounted forward, a put the
/// discounted strike. A quote outside this range cannot be inverted, and
/// knowing which side it fell out on is what makes the error actionable.
fn price_bounds(inputs: &BlackScholesInputs) -> (f64, f64) {
    let forward = inputs.spot * (-inputs.dividend_yield * inputs.time_to_expiry).exp();
    let discounted_strike = inputs.strike * (-inputs.rate * inputs.time_to_expiry).exp();
    match inputs.option_type {
        OptionType::Call => ((forward - discounted_strike).max(0.0), forward),
        OptionType::Put => ((discounted_strike - forward).max(0.0), discounted_strike),
    }
}

/// Solve for the volatility that reproduces `market_price`.
///
/// Newton–Raphson from a Manaster–Koehler starting point, with bisection as
/// the fallback. Newton is fast but unreliable in the wings, where vega goes
/// to zero and a step divides by almost nothing; bisection cannot fail on a
/// bracketed monotone function, which price-in-volatility is. Using both is
/// what makes the solver both quick near the money and trustworthy away from
/// it.
///
/// The `volatility` field of `inputs` is ignored — it is what is being solved
/// for.
pub fn implied_volatility(
    inputs: &BlackScholesInputs,
    market_price: f64,
) -> Result<f64, ImpliedVolError> {
    if !inputs.spot.is_finite()
        || inputs.spot <= 0.0
        || !inputs.strike.is_finite()
        || inputs.strike <= 0.0
        || !inputs.time_to_expiry.is_finite()
        || !inputs.rate.is_finite()
        || !inputs.dividend_yield.is_finite()
    {
        return Err(ImpliedVolError::MalformedInputs);
    }
    if !market_price.is_finite() || market_price <= 0.0 {
        return Err(ImpliedVolError::MalformedPrice);
    }
    if inputs.time_to_expiry < MIN_TIME {
        return Err(ImpliedVolError::Expired);
    }

    let (lower_bound, upper_bound) = price_bounds(inputs);
    // A hair of tolerance: a quote exactly at intrinsic implies zero vol, and
    // floating-point equality on a discounted difference is not something to
    // rely on.
    let price_epsilon = PRICE_TOLERANCE * market_price.max(1.0);
    if market_price < lower_bound - price_epsilon {
        return Err(ImpliedVolError::BelowIntrinsic);
    }
    if market_price >= upper_bound {
        return Err(ImpliedVolError::AboveMaximum);
    }

    // Takes the base inputs by value so it does not borrow the mutable probe
    // the Newton loop walks.
    let price_at = |vol: f64| -> Option<f64> {
        let mut candidate = *inputs;
        candidate.volatility = vol;
        price(&candidate)
    };
    // If the floor of the search range already reproduces the quote, then so
    // does everything just above it: the price is flat in volatility here and
    // no solver can distinguish one value from another.
    let floor_price = price_at(IV_LOWER).ok_or(ImpliedVolError::MalformedInputs)?;
    if (market_price - floor_price).abs() <= price_epsilon {
        return Err(ImpliedVolError::Unidentifiable);
    }

    let mut probe = *inputs;

    // Manaster–Koehler seed: exact for an at-the-money forward option, and a
    // good starting point elsewhere.
    let seed = ((inputs.spot / inputs.strike).ln() + inputs.rate * inputs.time_to_expiry)
        .abs()
        .mul_add(2.0, 0.0)
        .max(1e-4)
        .sqrt()
        / inputs.time_to_expiry.sqrt();
    let mut vol = seed.clamp(IV_LOWER, IV_UPPER);

    for _ in 0..NEWTON_STEPS {
        probe.volatility = vol;
        let Some(g) = greeks(&probe) else {
            break;
        };
        let error = g.price - market_price;
        if error.abs() <= price_epsilon {
            return Ok(vol);
        }
        // Vega below this makes the Newton step meaningless — the price is
        // flat in volatility here, so a step would be enormous and arbitrary.
        if g.vega < 1e-10 {
            break;
        }
        let next = vol - error / g.vega;
        if !next.is_finite() || next <= IV_LOWER || next >= IV_UPPER {
            break;
        }
        if (next - vol).abs() < 1e-12 {
            return Ok(next);
        }
        vol = next;
    }

    // Bisection. Price is monotonically increasing in volatility, so if the
    // target lies between the endpoints it is reachable.
    let mut low = IV_LOWER;
    let mut high = IV_UPPER;
    let high_price = price_at(high).ok_or(ImpliedVolError::MalformedInputs)?;
    if market_price < floor_price || market_price > high_price {
        return Err(ImpliedVolError::OutsideSearchBounds);
    }

    for _ in 0..BISECTION_STEPS {
        // Termination is on the bracket, not the price: a price match near
        // zero would accept almost any volatility in the wings.
        if (high - low) <= VOL_TOLERANCE {
            break;
        }
        let mid = 0.5 * (low + high);
        let Some(mid_price) = price_at(mid) else {
            return Err(ImpliedVolError::MalformedInputs);
        };
        if mid_price < market_price {
            low = mid;
        } else {
            high = mid;
        }
    }
    Ok(0.5 * (low + high))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(spot: f64, strike: f64, t: f64, vol: f64) -> BlackScholesInputs {
        BlackScholesInputs {
            spot,
            strike,
            time_to_expiry: t,
            rate: 0.05,
            dividend_yield: 0.0,
            volatility: vol,
            option_type: OptionType::Call,
        }
    }

    fn put(spot: f64, strike: f64, t: f64, vol: f64) -> BlackScholesInputs {
        BlackScholesInputs {
            option_type: OptionType::Put,
            ..call(spot, strike, t, vol)
        }
    }

    #[test]
    fn the_normal_cdf_matches_known_values() {
        // Tight tolerances on purpose: a series approximation good to 1e-7
        // would pass a loose test and then show as a kink in a vol surface.
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-15);
        assert!((norm_cdf(1.0) - 0.841_344_746_068_543).abs() < 1e-12);
        assert!((norm_cdf(-1.0) - 0.158_655_253_931_457).abs() < 1e-12);
        assert!((norm_cdf(1.96) - 0.975_002_104_851_780).abs() < 1e-12);
        assert!((norm_cdf(-3.0) - 0.001_349_898_031_630).abs() < 1e-12);
        // Symmetry must hold exactly enough not to bias a skew calculation.
        for x in [0.3_f64, 1.1, 2.7, 5.5, 8.0] {
            assert!((norm_cdf(x) + norm_cdf(-x) - 1.0).abs() < 1e-14, "x = {x}");
        }
    }

    #[test]
    fn a_known_textbook_price_is_reproduced() {
        // S=100, K=100, T=1, r=5%, q=0, σ=20% → call ≈ 10.4506, put ≈ 5.5735.
        let c = price(&call(100.0, 100.0, 1.0, 0.20)).expect("call");
        let p = price(&put(100.0, 100.0, 1.0, 0.20)).expect("put");
        assert!((c - 10.450_583_572_185_565).abs() < 1e-10, "call {c}");
        assert!((p - 5.573_526_022_256_971).abs() < 1e-10, "put {p}");
    }

    #[test]
    fn put_call_parity_holds() {
        // C - P = S·e^{-qT} - K·e^{-rT}. Parity is the single best check that
        // the two branches share a consistent set of conventions.
        for (s, k, t, vol, q) in [
            (100.0, 100.0, 1.0, 0.2, 0.0),
            (87.5, 120.0, 0.25, 0.65, 0.02),
            (250.0, 200.0, 2.0, 0.15, 0.035),
            (10.0, 10.5, 0.08, 0.9, 0.0),
        ] {
            let mut c = call(s, k, t, vol);
            c.dividend_yield = q;
            let mut p = put(s, k, t, vol);
            p.dividend_yield = q;
            let lhs = price(&c).unwrap() - price(&p).unwrap();
            let rhs = s * (-q * t).exp() - k * (c.rate * -t).exp();
            assert!((lhs - rhs).abs() < 1e-10, "parity broke at {s}/{k}/{t}");
        }
    }

    #[test]
    fn greeks_agree_with_numerical_derivatives() {
        let base = call(100.0, 105.0, 0.75, 0.28);
        let g = greeks(&base).expect("greeks");
        let h = 1e-5;

        let bump = |f: &dyn Fn(&mut BlackScholesInputs)| {
            let mut up = base;
            f(&mut up);
            price(&up).unwrap()
        };

        let up_spot = bump(&|i: &mut BlackScholesInputs| i.spot += h);
        let down_spot = bump(&|i: &mut BlackScholesInputs| i.spot -= h);
        let numeric_delta = (up_spot - down_spot) / (2.0 * h);
        assert!((g.delta - numeric_delta).abs() < 1e-6, "delta {}", g.delta);

        let numeric_gamma = (up_spot - 2.0 * price(&base).unwrap() + down_spot) / (h * h);
        assert!((g.gamma - numeric_gamma).abs() < 1e-3, "gamma {}", g.gamma);

        let up_vol = bump(&|i: &mut BlackScholesInputs| i.volatility += h);
        let down_vol = bump(&|i: &mut BlackScholesInputs| i.volatility -= h);
        assert!(
            (g.vega - (up_vol - down_vol) / (2.0 * h)).abs() < 1e-5,
            "vega {}",
            g.vega
        );

        let up_rate = bump(&|i: &mut BlackScholesInputs| i.rate += h);
        let down_rate = bump(&|i: &mut BlackScholesInputs| i.rate -= h);
        assert!(
            (g.rho - (up_rate - down_rate) / (2.0 * h)).abs() < 1e-5,
            "rho {}",
            g.rho
        );

        // Theta is ∂price/∂time; shortening the time to expiry is -∂t.
        let shorter = bump(&|i: &mut BlackScholesInputs| i.time_to_expiry -= h);
        let longer = bump(&|i: &mut BlackScholesInputs| i.time_to_expiry += h);
        assert!(
            (g.theta - (shorter - longer) / (2.0 * h)).abs() < 1e-5,
            "theta {}",
            g.theta
        );
    }

    #[test]
    fn implied_volatility_round_trips() {
        // The property that matters: price at a vol, imply it back, get the
        // same vol. Across the wings, where Newton alone would fail.
        for vol in [0.05_f64, 0.12, 0.25, 0.60, 1.5, 3.0] {
            for (s, k, t) in [
                (100.0, 100.0, 1.0),
                (100.0, 150.0, 0.5),
                (100.0, 60.0, 0.25),
                (100.0, 100.0, 0.02),
                (4000.0, 3800.0, 1.5),
            ] {
                for inputs in [call(s, k, t, vol), put(s, k, t, vol)] {
                    let market = price(&inputs).unwrap();
                    match implied_volatility(&inputs, market) {
                        Ok(solved) => assert!(
                            (solved - vol).abs() < 1e-6,
                            "vol {vol} came back {solved} at {s}/{k}/{t}",
                        ),
                        // Legitimate: an option whose time value has rounded
                        // away against its intrinsic, or one worth almost
                        // nothing, prices identically across a band of
                        // volatilities. Refusing is the correct answer.
                        Err(ImpliedVolError::Unidentifiable) => {},
                        Err(other) => {
                            panic!("{other:?} at vol {vol} {s}/{k}/{t}, price {market}")
                        },
                    }
                }
            }
        }
    }

    #[test]
    fn a_price_with_no_time_value_is_reported_as_unidentifiable() {
        // A deep in-the-money put whose time value has rounded away against
        // its intrinsic. Every volatility from zero up to roughly 8% produces
        // this same price in f64, so there is no unique answer — and picking
        // one would seed a surface with a number nobody can reproduce.
        let inputs = put(100.0, 150.0, 0.5, 0.05);
        let market = price(&inputs).expect("price");
        assert_eq!(
            implied_volatility(&inputs, market),
            Err(ImpliedVolError::Unidentifiable),
        );

        // Same refusal from the other end: a far out-of-the-money call worth
        // about 1e-27.
        let worthless = call(100.0, 150.0, 0.5, 0.05);
        let market = price(&worthless).expect("price");
        assert_eq!(
            implied_volatility(&worthless, market),
            Err(ImpliedVolError::Unidentifiable),
        );

        // But a contract with real time value at the same strike still
        // solves, so the guard is not simply refusing everything far from the
        // money.
        let solvable = put(100.0, 150.0, 0.5, 0.45);
        let market = price(&solvable).expect("price");
        let solved = implied_volatility(&solvable, market).expect("solvable");
        assert!((solved - 0.45).abs() < 1e-6, "{solved}");
    }

    #[test]
    fn a_quote_below_intrinsic_has_no_implied_volatility() {
        // The case that matters most. A deep ITM call marked below intrinsic
        // is arbitrageable and unsolvable; returning a small volatility here
        // would put a fabricated point into the surface.
        let inputs = call(100.0, 50.0, 1.0, 0.2);
        let (intrinsic, _) = price_bounds(&inputs);
        assert!(intrinsic > 0.0);
        assert_eq!(
            implied_volatility(&inputs, intrinsic - 1.0),
            Err(ImpliedVolError::BelowIntrinsic),
        );
    }

    #[test]
    fn an_impossible_quote_is_rejected_on_the_right_side() {
        let c = call(100.0, 100.0, 1.0, 0.2);
        // No volatility makes a call worth more than the discounted forward.
        assert_eq!(
            implied_volatility(&c, 100.0),
            Err(ImpliedVolError::AboveMaximum),
        );
        assert_eq!(
            implied_volatility(&c, 0.0),
            Err(ImpliedVolError::MalformedPrice),
        );
        assert_eq!(
            implied_volatility(&c, f64::NAN),
            Err(ImpliedVolError::MalformedPrice),
        );

        let mut expired = c;
        expired.time_to_expiry = 0.0;
        assert_eq!(
            implied_volatility(&expired, 5.0),
            Err(ImpliedVolError::Expired)
        );
    }

    #[test]
    fn malformed_inputs_return_none_rather_than_nan() {
        // A NaN price propagates silently through a surface fit; a None
        // cannot be mistaken for a number.
        for bad in [
            BlackScholesInputs {
                spot: -1.0,
                ..call(100.0, 100.0, 1.0, 0.2)
            },
            BlackScholesInputs {
                strike: 0.0,
                ..call(100.0, 100.0, 1.0, 0.2)
            },
            BlackScholesInputs {
                volatility: -0.2,
                ..call(100.0, 100.0, 1.0, 0.2)
            },
            BlackScholesInputs {
                time_to_expiry: -1.0,
                ..call(100.0, 100.0, 1.0, 0.2)
            },
            BlackScholesInputs {
                spot: f64::NAN,
                ..call(100.0, 100.0, 1.0, 0.2)
            },
        ] {
            assert!(price(&bad).is_none(), "{bad:?} should not price");
        }
    }

    #[test]
    fn an_expired_option_is_worth_its_intrinsic_value() {
        let itm = BlackScholesInputs {
            time_to_expiry: 0.0,
            ..call(120.0, 100.0, 0.0, 0.3)
        };
        let g = greeks(&itm).expect("greeks");
        assert!((g.price - 20.0).abs() < 1e-12);
        assert_eq!(g.delta, 1.0);
        // Nothing uncertain remains, so there is no convexity and no decay.
        assert_eq!(g.gamma, 0.0);
        assert_eq!(g.vega, 0.0);
        assert_eq!(g.theta, 0.0);

        let otm = BlackScholesInputs {
            time_to_expiry: 0.0,
            ..call(80.0, 100.0, 0.0, 0.3)
        };
        let g = greeks(&otm).expect("greeks");
        assert_eq!(g.price, 0.0);
        assert_eq!(g.delta, 0.0);
    }

    #[test]
    fn price_increases_monotonically_with_volatility() {
        // The property bisection depends on. If it ever failed, the solver
        // could bracket a target it cannot reach.
        for inputs in [call(100.0, 110.0, 0.5, 0.0), put(100.0, 90.0, 0.5, 0.0)] {
            let mut previous = f64::NEG_INFINITY;
            for step in 1..200 {
                let mut probe = inputs;
                probe.volatility = f64::from(step) * 0.02;
                let value = price(&probe).unwrap();
                assert!(value > previous, "not monotone at σ={}", probe.volatility);
                previous = value;
            }
        }
    }

    #[test]
    fn gamma_and_vega_are_identical_for_calls_and_puts() {
        // Put–call parity again: the difference is linear in spot and has no
        // volatility exposure, so the convexity greeks must match exactly.
        let c = greeks(&call(100.0, 95.0, 0.4, 0.33)).unwrap();
        let p = greeks(&put(100.0, 95.0, 0.4, 0.33)).unwrap();
        assert!((c.gamma - p.gamma).abs() < 1e-15);
        assert!((c.vega - p.vega).abs() < 1e-15);
        // Delta differs by exactly the discounted dividend factor, which is 1
        // here since q = 0.
        assert!((c.delta - p.delta - 1.0).abs() < 1e-12);
    }
}
