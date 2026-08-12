"""
PRISMATIK Voice Service — STT/TTS using faster-whisper and edge-tts.
Port: 8767
"""
import os
import io
import base64
import tempfile
import logging
from flask import Flask, request, jsonify
from flask_cors import CORS

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("voice-service")

app = Flask(__name__)
CORS(app)

# STT model (loaded on first use)
_stt_model = None

def get_stt_model():
    global _stt_model
    if _stt_model is None:
        try:
            from faster_whisper import WhisperModel
            model_size = os.environ.get("WHISPER_MODEL", "base")
            logger.info(f"Loading Whisper model: {model_size}")
            _stt_model = WhisperModel(model_size, device="cpu", compute_type="int8")
            logger.info("Whisper model loaded")
        except Exception as e:
            logger.error(f"Failed to load Whisper: {e}")
            return None
    return _stt_model


@app.route("/stt", methods=["POST"])
def speech_to_text():
    """Transcribe audio to text."""
    data = request.json
    audio_b64 = data.get("audio", "")
    language = data.get("language", None)

    if not audio_b64:
        return jsonify({"error": "No audio data"}), 400

    model = get_stt_model()
    if model is None:
        return jsonify({"error": "STT model not available"}), 503

    try:
        # decode base64 audio
        audio_bytes = base64.b64decode(audio_b64)

        # write to temp file
        with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as f:
            f.write(audio_bytes)
            tmp_path = f.name

        # transcribe
        segments, info = model.transcribe(tmp_path, language=language, beam_size=5)
        text = " ".join([s.text for s in segments])

        os.unlink(tmp_path)

        return jsonify({
            "text": text.strip(),
            "language": info.language,
            "confidence": 1.0 - info.language_probability if hasattr(info, 'language_probability') else 0.8,
        })
    except Exception as e:
        return jsonify({"error": str(e)}), 500


@app.route("/tts", methods=["POST"])
def text_to_speech():
    """Convert text to speech using edge-tts (free, high quality)."""
    data = request.json
    text = data.get("text", "")
    voice = data.get("voice", "en-US-AriaNeural")
    rate = data.get("rate", "+0%")

    if not text:
        return jsonify({"error": "No text"}), 400

    try:
        import edge_tts
        import asyncio

        async def generate():
            communicate = edge_tts.Communicate(text, voice, rate=rate)
            audio_data = b""
            async for chunk in communicate.stream():
                if chunk["type"] == "audio":
                    audio_data += chunk["data"]
            return audio_data

        audio_data = asyncio.run(generate())
        audio_b64 = base64.b64encode(audio_data).decode()

        return jsonify({
            "audio_base64": audio_b64,
            "format": "mp3",
            "voice": voice,
        })
    except ImportError:
        return jsonify({"error": "edge-tts not installed. Run: pip install edge-tts"}), 503
    except Exception as e:
        return jsonify({"error": str(e)}), 500


@app.route("/voices", methods=["GET"])
def list_voices():
    """List available TTS voices."""
    try:
        import edge_tts
        import asyncio

        async def get_voices():
            voices = await edge_tts.list_voices()
            return [{"name": v["ShortName"], "language": v["Locale"], "gender": v["Gender"]} for v in voices[:50]]

        voices = asyncio.run(get_voices())
        return jsonify({"voices": voices})
    except:
        return jsonify({"voices": [
            {"name": "en-US-AriaNeural", "language": "en-US", "gender": "Female"},
            {"name": "en-US-GuyNeural", "language": "en-US", "gender": "Male"},
            {"name": "en-GB-SoniaNeural", "language": "en-GB", "gender": "Female"},
            {"name": "ja-JP-NanamiNeural", "language": "ja-JP", "gender": "Female"},
            {"name": "zh-CN-XiaoxiaoNeural", "language": "zh-CN", "gender": "Female"},
        ]})


@app.route("/health", methods=["GET"])
def health():
    return jsonify({"status": "ok", "stt": _stt_model is not None})


if __name__ == "__main__":
    port = int(os.environ.get("VOICE_PORT", 8767))
    logger.info(f"Voice service on port {port}")
    app.run(host="127.0.0.1", port=port, debug=False)
