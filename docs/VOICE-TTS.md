# Voice cloning & TTS (opt-in)

Speak text in an enrolled user's voice. **Consent-first**: a person opts in with
`/voice enroll` and provides a sample; nothing is captured without that.

## Pieces

```
 Discord ──▶ bot ──(reference clip)──▶  [ voices/ shared volume ]
                │                              ▲
                │  POST /synthesize            │ reads <id>.wav
                ▼  {voice_id, text}            │
        ┌───────────────────────┐             │
        │  tts  (Python + XTTS) │─────────────┘
        │  on the GPU           │  returns spoken WAV
        └───────────────────────┘
                │
   bot plays the WAV back into the voice channel (songbird)
```

- **`tts/`** — FastAPI service wrapping **XTTS-v2** (Coqui). `POST /synthesize`
  reads `voices/<voice_id>.wav` as the speaker reference and returns audio.
- **`voices` volume** — reference clips, shared read/write by `bot`, read by `tts`.
- **`voice_profiles` table** — `discord_id`, `consent_at`, `enrolled` (who's set up).
- **Bot commands** — `/voice enroll` (attach a clip), `/voice clear`, `/voice list`,
  `/tts user:@x text:"…"`.

## Enrollment: v1 = upload, v2 = live capture

- **v1 (reliable, ships first):** `/voice enroll` with an **audio attachment**
  (~15s). The bot downloads it, transcodes to mono 24 kHz WAV with ffmpeg, writes
  `voices/<discord_id>.wav`, records consent.
- **v2 (the "grab it from chat" version):** capture live via songbird's `receive`
  feature — register a global handler for `CoreEvent::SpeakingStateUpdate`
  (SSRC↔user map) + `CoreEvent::VoiceTick` (`decoded_voice: Vec<i16>` per SSRC),
  accumulate ~15s of the enrolling user, write the same WAV. Requires building
  songbird with `DecodeMode::Decode` (extra per-packet CPU) — add when v1 works.

## Deploying the GPU service

The `tts` image is **built on the GPU host** (it's huge; not via CI). Add to the
stack:

```yaml
  tts:
    build:
      context: .
      dockerfile: Dockerfile.tts
    container_name: nigerian-tts
    restart: unless-stopped
    environment:
      VOICES_DIR: /voices
    volumes:
      - voices:/voices
      - tts_models:/root/.local/share/tts   # model cache (~2 GB, first run)
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]

  bot:
    # ...add:
    volumes:
      - voices:/voices

volumes:
  voices:
  tts_models:
```

The bot reaches it at `http://tts:8001` (set `TTS_URL`). Requires the NVIDIA
Container Toolkit on the host.

## Licensing & ethics

- XTTS-v2 uses Coqui's **non-commercial** model license — fine for personal use.
- Voice cloning can impersonate; keep it **opt-in** and let people remove their
  profile (`/voice clear`). Consider restricting `/tts` to enrolled users / a role.
