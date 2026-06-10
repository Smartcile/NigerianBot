"""NigerianBot TTS service — voice cloning via Coqui XTTS-v2.

POST /synthesize {voice_id, text, language} → spoken WAV in that voice.
The reference clip lives at $VOICES_DIR/<voice_id>.wav (written by the bot on
enrollment). Zero-shot cloning: no per-user training, just the reference clip.
"""

import os
import tempfile

# Accept the Coqui model license non-interactively (non-commercial use).
os.environ.setdefault("COQUI_TOS_AGREED", "1")

import torch  # noqa: E402
from fastapi import FastAPI, HTTPException  # noqa: E402
from fastapi.responses import FileResponse  # noqa: E402
from pydantic import BaseModel  # noqa: E402
from TTS.api import TTS  # noqa: E402

VOICES_DIR = os.environ.get("VOICES_DIR", "/voices")
MODEL = os.environ.get("TTS_MODEL", "tts_models/multilingual/multi-dataset/xtts_v2")

device = "cuda" if torch.cuda.is_available() else "cpu"
print(f"[tts] loading {MODEL} on {device}", flush=True)
tts = TTS(MODEL).to(device)

app = FastAPI(title="NigerianBot TTS")


class SynthRequest(BaseModel):
    voice_id: str
    text: str
    language: str = "en"


@app.get("/health")
def health():
    return {"status": "ok", "device": device, "model": MODEL}


@app.post("/synthesize")
def synthesize(req: SynthRequest):
    # Guard against path traversal in voice_id.
    if not req.voice_id.isalnum():
        raise HTTPException(status_code=400, detail="invalid voice_id")
    reference = os.path.join(VOICES_DIR, f"{req.voice_id}.wav")
    if not os.path.isfile(reference):
        raise HTTPException(status_code=404, detail="voice not enrolled")
    text = req.text.strip()
    if not text:
        raise HTTPException(status_code=400, detail="empty text")
    if len(text) > 600:
        text = text[:600]

    out = tempfile.NamedTemporaryFile(suffix=".wav", delete=False)
    out.close()
    try:
        tts.tts_to_file(
            text=text,
            speaker_wav=reference,
            language=req.language,
            file_path=out.name,
        )
    except Exception as e:  # noqa: BLE001
        raise HTTPException(status_code=500, detail=f"synthesis failed: {e}")
    return FileResponse(out.name, media_type="audio/wav", filename="speech.wav")
