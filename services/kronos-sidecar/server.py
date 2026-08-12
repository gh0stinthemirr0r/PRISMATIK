"""
Kronos Sidecar Service — Financial K-line Foundation Model Inference

Runs as a standalone HTTP service on port 8766.
Loads Kronos models from HuggingFace and serves predictions.

Usage:
    pip install -r requirements.txt
    python server.py

Endpoints:
    POST /predict — single symbol prediction
    POST /batch  — batch prediction
    GET  /health  — health check
    GET  /models  — available models
"""

import os
import sys
import time
import logging
from pathlib import Path

import numpy as np
import pandas as pd
from flask import Flask, request, jsonify

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("kronos-sidecar")

app = Flask(__name__)

# Global model cache
_model_cache = {}

def get_model(model_name: str = "base"):
    """Load and cache a Kronos model."""
    if model_name in _model_cache:
        return _model_cache[model_name]

    try:
        # Try importing Kronos
        sys.path.insert(0, str(Path(__file__).parent / "kronos"))
        from model.kronos import Kronos, KronosPredictor

        model_map = {
            "mini": "NeoQuasar/Kronos-mini-base",
            "small": "NeoQuasar/Kronos-small-base",
            "base": "NeoQuasar/Kronos-base-base",
        }

        hf_id = model_map.get(model_name, model_map["base"])
        logger.info(f"Loading Kronos model: {hf_id}")

        predictor = KronosPredictor.from_pretrained(hf_id)
        _model_cache[model_name] = predictor
        logger.info(f"Model {model_name} loaded successfully")
        return predictor

    except ImportError as e:
        logger.error(f"Kronos not installed: {e}")
        logger.error("Install with: pip install -r requirements.txt")
        return None
    except Exception as e:
        logger.error(f"Failed to load model {model_name}: {e}")
        return None


def prepare_dataframe(prices: list, timestamps: list) -> pd.DataFrame:
    """Convert price/timestamp lists to a DataFrame Kronos expects."""
    df = pd.DataFrame({
        "timestamp": pd.to_datetime(timestamps) if timestamps else pd.date_range(
            start="2025-01-01", periods=len(prices), freq="D"
        ),
        "open": prices,
        "high": [p * 1.005 for p in prices],  # approx if no OHLCV
        "low": [p * 0.995 for p in prices],
        "close": prices,
        "volume": [0.0] * len(prices),
    })
    df = df.set_index("timestamp")
    return df


FLAT_BAND_BPS = 5.0  # must match prismatik_regime::FLAT_BAND_BPS


def terminal_probabilities(all_predictions, last_price):
    """Directional probabilities from the model's own sampling distribution.

    Kronos is sampled `sample_count` times; each path has a terminal close.
    The fraction of those paths finishing outside the flat band IS the model's
    probability — no distributional assumption is needed, and none is made.
    Deriving a probability from the confidence bands instead would mean
    assuming a shape the sampler never claimed.

    The band matches the Rust side exactly, because a probability measured
    against a different threshold than the one the resolver scores would be
    the wrong number no matter how carefully it was computed.
    """
    if not all_predictions or last_price <= 0:
        return None
    terminal = []
    for path in all_predictions:
        if len(path) == 0:
            continue
        terminal.append(float(path.iloc[-1]["close"]))
    if not terminal:
        return None
    moves_bps = [(close / last_price - 1.0) * 10_000.0 for close in terminal]
    n = float(len(moves_bps))
    up = sum(1 for m in moves_bps if m > FLAT_BAND_BPS) / n
    down = sum(1 for m in moves_bps if m < -FLAT_BAND_BPS) / n
    return {
        "probability_up": up,
        "probability_down": down,
        "probability_flat": max(0.0, 1.0 - up - down),
        "terminal_paths": len(terminal),
        "median_move_bps": float(np.median(moves_bps)),
    }


def statistical_fallback(prices: list, pred_len: int) -> dict:
    """Statistical fallback when Kronos is not available."""
    last_price = prices[-1]
    mean = np.mean(prices)
    std = np.std(prices)

    # mean-reversion with uncertainty
    predicted_bars = []
    for i in range(pred_len):
        t = i + 1
        reversion = (mean - last_price) * 0.05 * t
        noise = std * 0.1 * np.sqrt(t)
        close = last_price + reversion
        predicted_bars.append({
            "timestamp": f"+{t}",
            "open": close - noise * 0.3,
            "high": close + noise,
            "low": close - noise,
            "close": close,
            "volume": 0.0,
        })

    confidence_bands = []
    for i, bar in enumerate(predicted_bars):
        widen = std * np.sqrt(i + 1) * 0.15
        confidence_bands.append({
            "step": i + 1,
            "close_lower": bar["close"] - widen * 1.96,
            "close_upper": bar["close"] + widen * 1.96,
            "close_median": bar["close"],
            "high_lower": bar["high"] - widen,
            "high_upper": bar["high"] + widen,
            "low_lower": bar["low"] - widen,
            "low_upper": bar["low"] + widen,
        })

    return {
        "predicted_bars": predicted_bars,
        "confidence_bands": confidence_bands,
        "model": "statistical_fallback",
        # No sampling distribution exists here — this is a mean-reversion
        # sketch, not a model. Saying so is what stops the caller filing it
        # as a scored forecast alongside real Kronos output.
        "probabilities": None,
        "is_fallback": True,
    }


@app.route("/predict", methods=["POST"])
def predict():
    """Run Kronos prediction for a single symbol."""
    data = request.json
    symbol = data.get("symbol", "UNKNOWN")
    prices = data.get("prices", [])
    timestamps = data.get("timestamps", [])
    pred_len = data.get("pred_len", 5)
    model_name = data.get("model", "base")
    sample_count = data.get("sample_count", 10)

    if len(prices) < 30:
        return jsonify({"error": "Need at least 30 data points"}), 400

    start = time.time()

    # try Kronos model
    predictor = get_model(model_name)

    if predictor is not None:
        try:
            df = prepare_dataframe(prices, timestamps)
            x_timestamp = df.index
            y_timestamp = pd.date_range(
                start=df.index[-1] + pd.Timedelta(days=1),
                periods=pred_len,
                freq="D"
            )

            # run prediction with multiple samples for confidence bands
            all_predictions = []
            for _ in range(sample_count):
                pred_df = predictor.predict(
                    df=df,
                    x_timestamp=x_timestamp,
                    y_timestamp=y_timestamp,
                    pred_len=pred_len,
                    T=1.0,
                    top_p=0.9,
                )
                all_predictions.append(pred_df)

            # aggregate predictions
            predicted_bars = []
            for i in range(pred_len):
                closes = [p.iloc[i]["close"] for p in all_predictions if len(p) > i]
                highs = [p.iloc[i]["high"] for p in all_predictions if len(p) > i]
                lows = [p.iloc[i]["low"] for p in all_predictions if len(p) > i]
                opens = [p.iloc[i]["open"] for p in all_predictions if len(p) > i]
                volumes = [p.iloc[i].get("volume", 0) for p in all_predictions if len(p) > i]

                predicted_bars.append({
                    "timestamp": str(y_timestamp[i]) if i < len(y_timestamp) else f"+{i+1}",
                    "open": float(np.median(opens)),
                    "high": float(np.median(highs)),
                    "low": float(np.median(lows)),
                    "close": float(np.median(closes)),
                    "volume": float(np.median(volumes)),
                })

            # confidence bands from sample distribution
            confidence_bands = []
            for i in range(pred_len):
                closes = [p.iloc[i]["close"] for p in all_predictions if len(p) > i]
                highs = [p.iloc[i]["high"] for p in all_predictions if len(p) > i]
                lows = [p.iloc[i]["low"] for p in all_predictions if len(p) > i]

                confidence_bands.append({
                    "step": i + 1,
                    "close_lower": float(np.percentile(closes, 5)),
                    "close_upper": float(np.percentile(closes, 95)),
                    "close_median": float(np.median(closes)),
                    "high_lower": float(np.percentile(highs, 5)),
                    "high_upper": float(np.percentile(highs, 95)),
                    "low_lower": float(np.percentile(lows, 5)),
                    "low_upper": float(np.percentile(lows, 95)),
                })

            inference_ms = int((time.time() - start) * 1000)

            return jsonify({
                "symbol": symbol,
                "predicted_bars": predicted_bars,
                "confidence_bands": confidence_bands,
                "model": f"Kronos-{model_name}",
                "inference_time_ms": inference_ms,
                "sample_count": sample_count,
                # Empirical, from the sampled paths. Absent rather than
                # guessed when the sampler produced nothing usable.
                "probabilities": terminal_probabilities(all_predictions, prices[-1]),
                "is_fallback": False,
            })

        except Exception as e:
            logger.error(f"Kronos prediction failed: {e}")
            # fall through to statistical fallback

    # statistical fallback
    result = statistical_fallback(prices, pred_len)
    result["symbol"] = symbol
    result["inference_time_ms"] = int((time.time() - start) * 1000)
    result["sample_count"] = 1
    return jsonify(result)


@app.route("/batch", methods=["POST"])
def batch_predict():
    """Run Kronos prediction for multiple symbols."""
    data = request.json
    symbols = data.get("symbols", [])
    price_data = data.get("data", [])
    pred_len = data.get("pred_len", 5)
    model_name = data.get("model", "base")

    results = []
    for symbol, (prices, timestamps) in zip(symbols, price_data):
        try:
            resp = predict.__wrapped__()  # internal call
        except:
            pass
        # simplified: call predict endpoint internally
        with app.test_client() as client:
            resp = client.post("/predict", json={
                "symbol": symbol,
                "prices": prices,
                "timestamps": timestamps,
                "pred_len": pred_len,
                "model": model_name,
            })
            results.append(resp.get_json())

    return jsonify({
        "predictions": results,
        "total": len(symbols),
        "successful": sum(1 for r in results if "error" not in r),
    })


@app.route("/health", methods=["GET"])
def health():
    """Health check."""
    models_loaded = list(_model_cache.keys())
    return jsonify({
        "status": "ok",
        "models_loaded": models_loaded,
        "available": ["mini", "small", "base"],
    })


@app.route("/models", methods=["GET"])
def models():
    """List available models."""
    return jsonify({
        "models": [
            {"id": "mini", "params": "4.1M", "context": 2048, "loaded": "mini" in _model_cache},
            {"id": "small", "params": "24.7M", "context": 512, "loaded": "small" in _model_cache},
            {"id": "base", "params": "102.3M", "context": 512, "loaded": "base" in _model_cache},
        ]
    })


if __name__ == "__main__":
    port = int(os.environ.get("KRONOS_PORT", 8766))
    logger.info(f"Starting Kronos sidecar on port {port}")
    logger.info("Models will be loaded on first request")
    app.run(host="127.0.0.1", port=port, debug=False)
