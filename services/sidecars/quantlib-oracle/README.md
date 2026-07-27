# QuantLib conformance sidecar

Offline-first Black-Scholes oracle for Wave 2 DoD #7.

## Usage

```bash
# Regenerate the 500-case grid (uses QuantLib if installed, else erf analytic)
python services/sidecars/quantlib-oracle/generate_fixture.py

# Gate (same as CI)
cargo test -p prismatik-quant-kernel quantlib_conformance_500_relative_1e8 -- --nocapture
```

Optional: `pip install QuantLib` to regenerate against the real C++ oracle.
When QuantLib is absent, fixtures are produced with an independent erf CDF
matching the QuantLib European analytic closed form.

Relative tolerance: `1e-8` on price and greeks (`max(|expected|, 1e-12)` scale).
