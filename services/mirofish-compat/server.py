"""
MiroFish-Compatible Multi-Agent Simulation Service
Works with Python 3.12 without camel-oasis dependency.
Uses OpenAI-compatible API for agent simulation.

Port: 5001 (same as original MiroFish)
"""
import os
import json
import time
import uuid
import logging
from datetime import datetime
from flask import Flask, request, jsonify
from flask_cors import CORS

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("mirofish")

app = Flask(__name__)
CORS(app)

# simulation state
simulations = {}
reports = {}

def get_llm_client():
    """Get OpenAI-compatible client."""
    from openai import OpenAI
    api_key = os.environ.get("OPENAI_API_KEY") or os.environ.get("QWEN_API_KEY", "")
    base_url = os.environ.get("LLM_BASE_URL", "https://api.openai.com/v1")
    if not api_key:
        # try Ollama
        return OpenAI(base_url="http://localhost:11434/v1", api_key="ollama"), "ollama"
    return OpenAI(base_url=base_url, api_key=api_key), "cloud"

AGENT_PERSONAS = [
    {"name": "Quant Analyst", "personality": "You are a quantitative analyst. You focus on statistical patterns, momentum, mean reversion, and mathematical models. You speak in numbers and probabilities."},
    {"name": "Fundamental Analyst", "personality": "You are a fundamental analyst. You focus on earnings, revenue, margins, competitive position, and intrinsic value. You cite specific financial metrics."},
    {"name": "Macro Strategist", "personality": "You are a macro strategist. You focus on interest rates, inflation, GDP, central bank policy, and geopolitical events. You think in terms of regimes."},
    {"name": "Technical Trader", "personality": "You are a technical trader. You focus on chart patterns, support/resistance, volume, and momentum indicators. You think in terms of entry/exit points."},
    {"name": "Risk Manager", "personality": "You are a risk manager. You focus on downside scenarios, tail risks, correlations, and position sizing. You are naturally skeptical and cautious."},
    {"name": "Sentiment Analyst", "personality": "You are a sentiment analyst. You focus on news flow, social media, market positioning, and crowd psychology. You read the tape."},
    {"name": "Contrarian", "personality": "You are a contrarian investor. You look for overextended moves, crowded trades, and mean reversion opportunities. You disagree with consensus."},
    {"name": "Momentum Trader", "personality": "You are a momentum trader. You follow the trend, cut losers quickly, and let winners run. You believe in relative strength."},
    {"name": "Value Investor", "personality": "You are a value investor. You look for undervalued assets with margin of safety. You think in years, not days."},
    {"name": "Options Specialist", "personality": "You are an options specialist. You focus on volatility, skew, term structure, and Greeks. You think probabilistically about outcomes."},
]

def run_simulation(sim_id, seed_docs, query, agent_count, rounds, market_context):
    """Run multi-agent simulation."""
    sim = simulations[sim_id]
    sim["state"] = "simulating"
    sim["progress"] = 0.1

    try:
        client, provider = get_llm_client()
        model = os.environ.get("LLM_MODEL", "gpt-4o-mini")
        if provider == "ollama":
            model = os.environ.get("OLLAMA_MODEL", "llama3.1")

        # prepare context from seed documents
        context = "\n".join([f"[{doc.get('source', 'unknown')}] {doc.get('title', '')}: {doc.get('content', '')}" for doc in seed_docs[:20]])
        market_info = json.dumps(market_context, indent=2) if market_context else "No market context available."

        agent_results = []
        personas = AGENT_PERSONAS[:min(agent_count, len(AGENT_PERSONAS))]

        for round_num in range(rounds):
            sim["rounds_completed"] = round_num + 1
            sim["progress"] = 0.1 + (round_num / rounds) * 0.7

            for persona in personas:
                try:
                    response = client.chat.completions.create(
                        model=model,
                        messages=[
                            {"role": "system", "content": f"{persona['personality']}\n\nYou are participating in a market simulation. Analyze the data and give your prediction.\n\nMarket Context:\n{market_info}\n\nNews/Data:\n{context}"},
                            {"role": "user", "content": f"Round {round_num + 1}: {query}\n\nGive your analysis in 2-3 sentences. State your direction (bullish/bearish/neutral) and conviction (0-1). Format: DIRECTION: [bullish/bearish/neutral] CONVICTION: [0-1] REASONING: [your analysis]"}
                        ],
                        max_tokens=200,
                        temperature=0.8,
                    )
                    text = response.choices[0].message.content

                    # parse direction and conviction
                    direction = "neutral"
                    conviction = 0.5
                    if "DIRECTION:" in text:
                        dir_part = text.split("DIRECTION:")[1].strip().split()[0].lower()
                        if "bull" in dir_part: direction = "bullish"
                        elif "bear" in dir_part: direction = "bearish"
                    if "CONVICTION:" in text:
                        try:
                            conv_part = text.split("CONVICTION:")[1].strip().split()[0]
                            conviction = float(conv_part)
                        except: pass

                    agent_results.append({
                        "persona": persona["name"],
                        "direction": direction,
                        "conviction": conviction,
                        "reasoning": text,
                        "round": round_num + 1,
                    })
                except Exception as e:
                    logger.error(f"Agent {persona['name']} round {round_num+1} failed: {e}")

        sim["progress"] = 0.9
        sim["state"] = "generating_report"

        # aggregate results
        bull_count = sum(1 for r in agent_results if r["direction"] == "bullish")
        bear_count = sum(1 for r in agent_results if r["direction"] == "bearish")
        neutral_count = sum(1 for r in agent_results if r["direction"] == "neutral")
        total = len(agent_results) or 1

        avg_conviction = sum(r["conviction"] for r in agent_results) / total if total else 0.5
        consensus_strength = max(bull_count, bear_count, neutral_count) / total

        if bull_count > bear_count and bull_count > neutral_count:
            consensus = "bullish"
        elif bear_count > bull_count and bear_count > neutral_count:
            consensus = "bearish"
        else:
            consensus = "neutral"

        # key arguments
        key_args = []
        for persona in AGENT_PERSONAS[:5]:
            persona_results = [r for r in agent_results if r["persona"] == persona["name"]]
            if persona_results:
                last = persona_results[-1]
                key_args.append({
                    "argument": f"{persona['name']}: {last['reasoning'][:200]}",
                    "supporting": 1 if last["direction"] == consensus else 0,
                    "opposing": 1 if last["direction"] != consensus else 0,
                    "strength": last["conviction"],
                })

        report = {
            "scenario_id": sim_id,
            "generated_at": datetime.utcnow().isoformat() + "Z",
            "summary": f"Multi-agent simulation with {len(personas)} personas over {rounds} rounds. Consensus: {consensus} ({consensus_strength:.0%} agreement).",
            "consensus_direction": consensus,
            "consensus_strength": consensus_strength,
            "agent_sentiments": [{"agent_persona": r["persona"], "direction": r["direction"], "conviction": r["conviction"], "reasoning": r["reasoning"][:200]} for r in agent_results[-20:]],
            "key_arguments": key_args,
            "emergent_themes": [f"{'Bullish' if bull_count > bear_count else 'Bearish'} bias from {max(bull_count, bear_count)}/{total} agents"],
            "risk_factors": [f"High disagreement: {neutral_count}/{total} neutral", f"Average conviction: {avg_conviction:.2f}"],
            "confidence": avg_conviction,
            "divergence_map": [{"topic": "Overall Direction", "bull_pct": bull_count/total, "bear_pct": bear_count/total, "neutral_pct": neutral_count/total}],
            "full_report": json.dumps(agent_results, indent=2),
        }

        reports[sim_id] = report
        sim["state"] = "complete"
        sim["progress"] = 1.0

    except Exception as e:
        logger.error(f"Simulation {sim_id} failed: {e}")
        sim["state"] = "error"
        sim["message"] = str(e)


@app.route("/api/graph/health", methods=["GET"])
@app.route("/", methods=["GET"])
def health():
    return jsonify({"status": "ok", "service": "mirofish-compatible", "version": "1.0.0"})


@app.route("/api/simulation/start", methods=["POST"])
def start_simulation():
    data = request.json
    sim_id = data.get("scenario_id", f"sim_{uuid.uuid4().hex[:8]}")
    seed_docs = data.get("seed_documents", [])
    query = data.get("prediction_query", "What is your market prediction?")
    agent_count = data.get("agent_count", 10)
    rounds = data.get("simulation_rounds", 3)
    market_context = data.get("market_context", {})

    simulations[sim_id] = {
        "scenario_id": sim_id,
        "state": "building_graph",
        "progress": 0.0,
        "agents_active": min(agent_count, len(AGENT_PERSONAS)),
        "rounds_completed": 0,
        "message": "Simulation starting",
    }

    # run in background thread
    import threading
    t = threading.Thread(target=run_simulation, args=(sim_id, seed_docs, query, agent_count, rounds, market_context))
    t.daemon = True
    t.start()

    return jsonify(simulations[sim_id])


@app.route("/api/simulation/status/<sim_id>", methods=["GET"])
def simulation_status(sim_id):
    if sim_id in simulations:
        return jsonify(simulations[sim_id])
    return jsonify({"error": "Simulation not found"}), 404


@app.route("/api/report/<sim_id>", methods=["GET"])
def get_report(sim_id):
    if sim_id in reports:
        return jsonify(reports[sim_id])
    return jsonify({"error": "Report not found"}), 404


if __name__ == "__main__":
    port = int(os.environ.get("MIROFISH_PORT", 5001))
    logger.info(f"MiroFish-compatible service on port {port}")
    app.run(host="127.0.0.1", port=port, debug=False, threaded=True)
