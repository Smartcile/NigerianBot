import { useState, useEffect, useCallback, useMemo, useRef } from 'react'

const TOKEN_KEY = 'nb_token'
const CHANGE_KEY = 'nb_must_change'

export default function App() {
  const [token, setToken] = useState(() => localStorage.getItem(TOKEN_KEY))
  const [mustChange, setMustChange] = useState(() => localStorage.getItem(CHANGE_KEY) === '1')

  const login = useCallback((t, mustChangePin) => {
    localStorage.setItem(TOKEN_KEY, t)
    localStorage.setItem(CHANGE_KEY, mustChangePin ? '1' : '0')
    setToken(t)
    setMustChange(!!mustChangePin)
  }, [])

  const logout = useCallback(() => {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(CHANGE_KEY)
    setToken(null)
  }, [])

  const pinChanged = useCallback(() => {
    localStorage.setItem(CHANGE_KEY, '0')
    setMustChange(false)
  }, [])

  if (!token) return <Login onLogin={login} />
  if (mustChange) return <ForceChangePin token={token} onDone={pinChanged} onLogout={logout} />
  return <Shell token={token} onLogout={logout} />
}

/* ── API helper ──────────────────────────────────────────────────────────── */

function makeApi(token, onLogout) {
  const auth = { Authorization: 'Bearer ' + token }
  const check = (r) => {
    if (r.status === 401) {
      onLogout()
      throw new Error('unauthorized')
    }
    return r
  }
  const json = (r) => (r.status === 204 ? null : r.json().catch(() => null))
  return {
    async get(path) {
      return json(check(await fetch(path, { headers: auth })))
    },
    async post(path, body) {
      return json(
        check(
          await fetch(path, {
            method: 'POST',
            headers: { ...auth, 'Content-Type': 'application/json' },
            body: JSON.stringify(body ?? {}),
          }),
        ),
      )
    },
    async postText(path, text) {
      return json(
        check(
          await fetch(path, {
            method: 'POST',
            headers: { ...auth, 'Content-Type': 'text/plain' },
            body: text,
          }),
        ),
      )
    },
    async del(path) {
      check(await fetch(path, { method: 'DELETE', headers: auth }))
    },
  }
}

/* ── Login ───────────────────────────────────────────────────────────────── */

function Login({ onLogin }) {
  const [pin, setPin] = useState('')
  const [err, setErr] = useState('')

  async function submit(e) {
    e.preventDefault()
    setErr('')
    try {
      const r = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pin }),
      })
      if (r.status === 401) {
        setErr('WRONG PIN. THIS TRANSACTION IS HIGHLY CONFIDENTIAL.')
        return
      }
      if (!r.ok) {
        setErr('SIGN-IN FAILED. PLEASE TRY AGAIN.')
        return
      }
      const data = await r.json()
      onLogin(data.token, data.must_change_pin)
    } catch {
      setErr('SERVER UNREACHABLE. PLEASE TRY AGAIN.')
    }
  }

  return (
    <div className="login">
      <div className="popup-note">🔒 100% GUARANTEED · NO RISK · GOD BLESS</div>
      <form className="card" onSubmit={submit}>
        <div className="seal-wrap">
          <Seal />
        </div>
        <h1>🇳🇬 NIGERIANBOT HEADQUARTERS</h1>
        <p className="scam-sub">
          STRICTLY CONFIDENTIAL BUSINESS PROPOSAL — KIND ATTENTION: DEAR FRIEND
        </p>
        <p className="letter">
          I am the system administrator of a late <b>Honourable Controller</b> and I have the
          privilege to move the sum of <b>US$45,000,000.00</b> (FOURTY FIVE MILLION UNITED STATES
          DOLLARS) into your trust account. To proceed, kindly furnish the undermentioned secret
          PIN.
        </p>
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="ENTER YOUR PIN (DEFAULT: 1234)"
          value={pin}
          onChange={(e) => setPin(e.target.value)}
        />
        <button type="submit">🤑 CLAIM ACCESS NOW!!!</button>
        <div className="err">{err}</div>
        <div className="fineprint">
          First time? Use PIN <b>1234</b>; you'll be asked to choose a new one.
        </div>
      </form>
    </div>
  )
}

/* Forced PIN change on first sign-in. */
function ForceChangePin({ token, onDone, onLogout }) {
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirm, setConfirm] = useState('')
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)

  async function submit(e) {
    e.preventDefault()
    setErr('')
    if (next !== confirm) {
      setErr('THE TWO PINS DO NOT MATCH.')
      return
    }
    setBusy(true)
    try {
      const r = await fetch('/api/auth/change-pin', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token },
        body: JSON.stringify({ current_pin: current, new_pin: next }),
      })
      if (r.status === 401) {
        setErr('CURRENT PIN IS WRONG.')
        setBusy(false)
        return
      }
      if (!r.ok) {
        setErr('PIN MUST BE 4-8 DIGITS.')
        setBusy(false)
        return
      }
      onDone()
    } catch {
      setErr('COULD NOT REACH THE SERVER.')
      setBusy(false)
    }
  }

  return (
    <div className="login">
      <div className="popup-note">🔐 SECURITY NOTICE</div>
      <form className="card" onSubmit={submit}>
        <div className="seal-wrap">
          <Seal />
        </div>
        <h1>🇳🇬 CHANGE YOUR SECRET PIN</h1>
        <p className="scam-sub">FIRST SIGN-IN — PROTECT YOUR US$45,000,000.00</p>
        <p className="letter">
          For your own protection, kindly replace the default PIN (<b>1234</b>) with a secret
          4-8 digit code that only you know. Do not disclose it to any person, including the
          Controller.
        </p>
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="CURRENT PIN (1234)"
          value={current}
          onChange={(e) => setCurrent(e.target.value)}
        />
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="NEW PIN (4-8 DIGITS)"
          value={next}
          onChange={(e) => setNext(e.target.value)}
        />
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="CONFIRM NEW PIN"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
        <button type="submit" disabled={busy}>
          {busy ? 'SAVING…' : '🔐 SAVE NEW PIN'}
        </button>
        <div className="err">{err}</div>
        <div className="fineprint">
          <span className="linklike" onClick={onLogout}>
            Sign out instead
          </span>
        </div>
      </form>
    </div>
  )
}

/* ── Shell ───────────────────────────────────────────────────────────────── */

const TABS = [
  ['overview', '💰 CLAIM FUNDS'],
  ['downloads', '📥 CARGO'],
  ['media', '🎬 BUSINESS LOTS'],
  ['schedules', '⏰ PAYMENT PLAN'],
  ['users', '👑 HONOURABLES'],
  ['telegram', '✉️ CABLEGRAM'],
  ['setup', '📖 SETUP'],
]

function Shell({ token, onLogout }) {
  const [tab, setTab] = useState('overview')
  const [popup, setPopup] = useState(true)
  const api = useMemo(() => makeApi(token, onLogout), [token, onLogout])

  return (
    <>
      <Ticker />
      {popup && (
        <div className="spam-popup">
          <button className="close" onClick={() => setPopup(false)}>
            ✕
          </button>
          <div className="spam-title">🎉 CONGRATULATIONS!!!</div>
          <div className="spam-body">
            You are the <b>1,000,000th</b> visitor of NigerianBot! You have won
            <b> US$45,000,000.00</b>. Click below and send only a small processing fee.
          </div>
          <button className="spam-cta" onClick={() => setPopup(false)}>
            💸 CLAIM PRIZE (FREE)
          </button>
        </div>
      )}

      <header className="top">
        <div className="brand">
          <span className="flag">🇳🇬</span>
          <span>
            NIGERIANBOT&nbsp;HEADQUARTERS
            <small>“Serving Honourable People Since 419 BC”</small>
          </span>
        </div>
        <div className="conf">★ CONFIDENTIAL ★</div>
        <button className="logout" onClick={onLogout}>
          LOG OUT
        </button>
      </header>

      <nav className="tabs">
        {TABS.map(([id, label]) => (
          <button
            key={id}
            className={'tab ' + (tab === id ? 'active' : '')}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </nav>

      <main>
        {tab === 'overview' && <Overview api={api} />}
        {tab === 'downloads' && <Downloads api={api} />}
        {tab === 'media' && <Media api={api} />}
        {tab === 'schedules' && <Schedules api={api} />}
        {tab === 'users' && <Users api={api} />}
        {tab === 'telegram' && <Telegram api={api} />}
        {tab === 'setup' && <Setup api={api} />}
      </main>

      <footer className="foot">
        ⚠️ STRICTLY CONFIDENTIAL — This document must not be disclosed. Kindly send processing
        fee of <b>US$500</b> to release your funds. GOD BLESS. — The Management
      </footer>
    </>
  )
}

/* ── Overview ────────────────────────────────────────────────────────────── */

function Overview({ api }) {
  const [status, setStatus] = useState(null)
  const [stats, setStats] = useState(null)
  const [logs, setLogs] = useState([])
  const [updated, setUpdated] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [st, stt, lg] = await Promise.all([
        api.get('/api/bot/status'),
        api.get('/api/stats'),
        api.get('/api/bot/logs?limit=25'),
      ])
      setStatus(st)
      setStats(stt)
      setLogs(lg?.logs || [])
      setUpdated(new Date().toLocaleTimeString())
    } catch {
      /* handled in api */
    }
  }, [api])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, 10000)
    return () => clearInterval(t)
  }, [refresh])

  const top = stats?.top || []
  const max = Math.max(1, ...top.map((t) => t.count))
  const dbOk = status?.database === 'connected'

  return (
    <>
      <div className="grid">
        <Stat label="TOTAL TRANSACTIONS" value={stats?.total ?? '—'} />
        <Stat label="LAST 24 HOURS" value={stats?.last_24h ?? '—'} />
        <Stat
          label="VAULT (DATABASE)"
          value={<span className={'pill ' + (dbOk ? 'done' : 'failed')}>{dbOk ? 'SECURE' : 'BREACHED'}</span>}
        />
        <Stat label="FUNDS ON HOLD" value="$45,000,000" />
      </div>

      <div className="two-col">
        <section className="section">
          <h2>💵 TOP COMMANDS (MONEY MOVERS)</h2>
          {top.length ? (
            top.map((t) => (
              <div className="bar-row" key={t.command}>
                <div className="name">
                  <code>/{t.command}</code>
                </div>
                <div className="bar">
                  <span style={{ width: (100 * t.count) / max + '%' }} />
                </div>
                <div className="n">{t.count}</div>
              </div>
            ))
          ) : (
            <div className="muted">No transactions yet, my friend.</div>
          )}
        </section>

        <AiImage label="CHIEF AUDITOR" caption="AI IMAGE SLOT — replace src" />
      </div>

      <section className="section">
        <h2>📜 RECENT TRANSACTIONS (AUDIT TRAIL)</h2>
        <table>
          <thead>
            <tr>
              <th>TIME</th>
              <th>HONOURABLE</th>
              <th>WIRE REFERENCE</th>
            </tr>
          </thead>
          <tbody>
            {logs.length ? (
              logs.map((l) => (
                <tr key={l.id}>
                  <td>{new Date(l.created_at).toLocaleString()}</td>
                  <td>{l.user_name || l.user_id}</td>
                  <td>
                    <code>/{l.command}</code>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="3" className="muted">
                  No transactions yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      <div className="updated">AUTO-REFRESH 10s · LAST SYNC {updated}</div>
    </>
  )
}

/* ── Downloads ───────────────────────────────────────────────────────────── */

function Downloads({ api }) {
  const [items, setItems] = useState([])
  const [url, setUrl] = useState('')
  const [saveAs, setSaveAs] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState('')
  const [importFile, setImportFile] = useState(null)

  const refresh = useCallback(async () => {
    try {
      const d = await api.get('/api/downloads?limit=50')
      setItems(d?.downloads || [])
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, 10000)
    return () => clearInterval(t)
  }, [refresh])

  async function submit(e) {
    e.preventDefault()
    if (!url.trim()) return
    setBusy(true)
    setNote('')
    try {
      const row = await api.post('/api/downloads', {
        url: url.trim(),
        save_as: saveAs.trim() || null,
      })
      setNote(row?.id ? `SUCCESS! Cargo #${row.id} is on the way.` : 'Submitted.')
      setUrl('')
      setSaveAs('')
      await refresh()
    } catch {
      setNote('TRANSACTION FAILED. Kindly try again.')
    }
    setBusy(false)
  }

  async function remove(id) {
    await api.del('/api/downloads/' + id)
    await refresh()
  }

  return (
    <>
      <section className="section highlight">
        <h2>📥 IMPORT CARGO (VIMEO / YOUTUBE / ANY URL)</h2>
        <form className="dlform" onSubmit={submit}>
          <input
            type="text"
            placeholder="PASTE THE CARGO URL (VIMEO / YOUTUBE / TUBI / ANY)"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
          />
          <input
            type="text"
            placeholder="SAVE AS (optional) e.g. Show - S01E05 - Title"
            value={saveAs}
            onChange={(e) => setSaveAs(e.target.value)}
          />
          <button type="submit" disabled={busy}>
            {busy ? 'SENDING…' : '💸 SHIP IT'}
          </button>
        </form>
        <div className="fineprint">
          A trusted agent (the worker) will collect the cargo and store it in the warehouse.
          {note ? <b> {note}</b> : null}
        </div>
      </section>

      <CookiesCard api={api} />

      <section className="section">
        <h2>🚢 CARGO MANIFEST</h2>
        <table>
          <thead>
            <tr>
              <th>TIME</th>
              <th>CARGO</th>
              <th>STATUS</th>
              <th>SIZE</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {items.length ? (
              items.map((d) => (
                <tr key={d.id}>
                  <td>{new Date(d.created_at).toLocaleString()}</td>
                  <td>
                    <div>{d.title || d.url}</div>
                    <div className="muted small">
                      {d.provider}
                      {d.requested_by ? ' · ' + d.requested_by : ''} · REF #{d.id}
                    </div>
                    {d.status === 'failed' && d.error ? (
                      <div className="err small">{d.error}</div>
                    ) : null}
                    {d.status === 'downloading' ? (
                      <div className="dlbar" title={(d.progress || 0) + '%'}>
                        <span style={{ width: Math.max(1, d.progress || 0) + '%' }} />
                      </div>
                    ) : null}
                  </td>
                  <td>
                    <span className={'pill ' + d.status}>{d.status}</span>
                    {d.status === 'downloading' ? (
                      <span className="pct"> {d.progress || 0}%</span>
                    ) : null}
                  </td>
                  <td>{d.size_bytes ? fmtSize(d.size_bytes) : '—'}</td>
                  <td className="row-actions">
                    {d.status === 'done' && d.file_path ? (
                      <a href={'/media/' + d.file_path} target="_blank" rel="noreferrer">
                        COLLECT
                      </a>
                    ) : null}
                    {d.status === 'done' && d.file_path ? (
                      <button className="link" onClick={() => setImportFile(d.file_path)}>
                        📥 IMPORT
                      </button>
                    ) : null}
                    <button className="link" onClick={() => remove(d.id)}>
                      DESTROY
                    </button>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="5" className="muted">
                  No cargo yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      {importFile ? (
        <ImportModal api={api} file={importFile} onClose={() => setImportFile(null)} />
      ) : null}
    </>
  )
}

/* Link a finished download to a library item and import via Sonarr/Radarr. */
function ImportModal({ api, file, onClose }) {
  const [service, setService] = useState('sonarr')
  const [data, setData] = useState(null)
  const [loading, setLoading] = useState(false)
  const [msg, setMsg] = useState('')
  const [linkTerm, setLinkTerm] = useState('')
  const [linkResults, setLinkResults] = useState([])
  const [chosen, setChosen] = useState(null)
  const [episodes, setEpisodes] = useState([])
  const [episodeId, setEpisodeId] = useState('')
  const [busy, setBusy] = useState(false)

  const load = useCallback(
    async (svc) => {
      setLoading(true)
      setMsg('')
      setData(null)
      setChosen(null)
      setEpisodes([])
      setLinkResults([])
      setLinkTerm('')
      setEpisodeId('')
      try {
        setData(await api.get(`/api/media/${svc}/manual-import?file=${encodeURIComponent(file)}`))
      } catch {
        setMsg('COULD NOT REACH THE MEDIA SERVICE.')
      }
      setLoading(false)
    },
    [api, file],
  )

  useEffect(() => {
    load(service)
  }, [load, service])

  const candidate = data?.candidates?.[0]

  function entryFor(c, override) {
    if (service === 'radarr') {
      return {
        path: c.path,
        movieId: override?.movieId ?? c.movie?.id,
        quality: c.quality,
        languages: c.languages,
      }
    }
    return {
      path: c.path,
      seriesId: override?.seriesId ?? c.series?.id,
      episodeIds: override?.episodeIds ?? (c.episodes || []).map((e) => e.id),
      quality: c.quality,
      languages: c.languages,
      releaseGroup: c.releaseGroup,
    }
  }

  async function doImport(files) {
    setBusy(true)
    setMsg('')
    try {
      await api.post(`/api/media/${service}/manual-import`, { import_mode: 'move', files })
      setMsg('✅ IMPORT STARTED — the library will rename & move it per your settings.')
    } catch {
      setMsg('IMPORT FAILED. CHECK THE MATCH AND TRY AGAIN.')
    }
    setBusy(false)
  }

  async function importProvided() {
    if (!candidate) return
    const entry = entryFor(candidate)
    if (service === 'sonarr' && (!entry.seriesId || !entry.episodeIds.length)) {
      setMsg("SONARR COULDN'T MATCH — LINK A SHOW/EPISODE BELOW.")
      return
    }
    if (service === 'radarr' && !entry.movieId) {
      setMsg("RADARR COULDN'T MATCH — SEARCH FOR THE MOVIE BELOW.")
      return
    }
    await doImport([entry])
  }

  async function searchLink(e) {
    e.preventDefault()
    setMsg('')
    try {
      const v = await api.get(`/api/media/${service}/search?term=${encodeURIComponent(linkTerm)}`)
      setLinkResults((v?.results || []).filter((r) => r.in_library))
    } catch {
      setMsg('SEARCH FAILED.')
    }
  }

  async function pickChosen(r) {
    setChosen(r)
    setEpisodeId('')
    if (service === 'sonarr') {
      try {
        const v = await api.get(`/api/media/sonarr/episodes?series_id=${r.id}`)
        setEpisodes(v?.episodes || [])
      } catch {
        setEpisodes([])
      }
    }
  }

  async function importLinked() {
    if (!candidate || !chosen) {
      setMsg('PICK A MATCH FIRST.')
      return
    }
    const override =
      service === 'radarr'
        ? { movieId: chosen.id }
        : { seriesId: chosen.id, episodeIds: episodeId ? [Number(episodeId)] : [] }
    if (service === 'sonarr' && !override.episodeIds.length) {
      setMsg('PICK AN EPISODE.')
      return
    }
    await doImport([entryFor(candidate, override)])
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <h2>📥 IMPORT “{file}”</h2>
          <button className="close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="tabs sub">
          {['sonarr', 'radarr'].map((s) => (
            <button
              key={s}
              className={'tab ' + (service === s ? 'active' : '')}
              onClick={() => setService(s)}
            >
              {s === 'radarr' ? '🎬 RADARR' : '📺 SONARR'}
            </button>
          ))}
        </div>

        {loading ? <div className="muted">LOADING…</div> : null}

        {!loading && data && !candidate ? (
          <div className="notice">
            {service.toUpperCase()} can't see this file. Make sure the shared folder is mounted into
            it and set{' '}
            <code>{service === 'radarr' ? 'RADARR_IMPORT_PATH' : 'SONARR_IMPORT_PATH'}</code>{' '}
            (currently <code>{data.import_path}</code>).
          </div>
        ) : null}

        {candidate ? (
          <>
            <div className="fineprint">
              FILE: <code>{candidate.path || candidate.name}</code>
              {candidate.quality?.quality?.name ? (
                <>
                  {' '}
                  · QUALITY: <b>{candidate.quality.quality.name}</b>
                </>
              ) : null}
            </div>

            {service === 'sonarr' ? (
              <div className="fineprint">
                MATCHED:{' '}
                {candidate.series?.title ? <b>{candidate.series.title}</b> : '— none —'}{' '}
                {candidate.episodes?.length
                  ? (candidate.episodes || [])
                      .map((e) => `S${pad(e.seasonNumber)}E${pad(e.episodeNumber)}`)
                      .join(', ')
                  : null}
              </div>
            ) : (
              <div className="fineprint">
                MATCHED:{' '}
                {candidate.movie?.title ? (
                  <b>
                    {candidate.movie.title} ({candidate.movie.year})
                  </b>
                ) : (
                  '— none —'
                )}
              </div>
            )}

            {candidate.rejections?.length ? (
              <div className="err small">
                {(candidate.rejections || []).map((r) => r.reason).join(' · ')}
              </div>
            ) : null}

            <button onClick={importProvided} disabled={busy} style={{ marginTop: 8 }}>
              ✅ IMPORT (USE THIS MATCH)
            </button>

            <hr />

            <div className="fineprint">
              LINK TO A DIFFERENT {service === 'radarr' ? 'MOVIE' : 'SHOW / EPISODE'} IN YOUR
              LIBRARY:
            </div>
            <form className="dlform" onSubmit={searchLink}>
              <input
                value={linkTerm}
                onChange={(e) => setLinkTerm(e.target.value)}
                placeholder="Search your library"
              />
              <button type="submit">🔎 SEARCH</button>
            </form>
            <div className="chips">
              {linkResults.map((r) => (
                <span
                  key={r.id}
                  className={'chip ' + (chosen?.id === r.id ? 'chip-on' : '')}
                  onClick={() => pickChosen(r)}
                >
                  {r.title} {r.year ? `(${r.year})` : ''}
                </span>
              ))}
            </div>

            {chosen && service === 'sonarr' ? (
              <select
                value={episodeId}
                onChange={(e) => setEpisodeId(e.target.value)}
                style={{ marginTop: 8 }}
              >
                <option value="">— PICK EPISODE —</option>
                {episodes.map((e) => (
                  <option key={e.id} value={e.id}>
                    S{pad(e.seasonNumber)}E{pad(e.episodeNumber)} — {e.title}
                  </option>
                ))}
              </select>
            ) : null}

            {chosen ? (
              <button onClick={importLinked} disabled={busy} style={{ marginTop: 8 }}>
                ✅ IMPORT INTO “{chosen.title}”
              </button>
            ) : null}
          </>
        ) : null}

        {msg ? <div className="notice">{msg}</div> : null}
      </div>
    </div>
  )
}

/* Login cookies for yt-dlp (Vimeo etc.). Uploaded here, not on the server. */
function CookiesCard({ api }) {
  const [status, setStatus] = useState(null)
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)
  const fileRef = useRef(null)

  const refresh = useCallback(async () => {
    try {
      setStatus(await api.get('/api/downloads/cookies'))
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function upload(e) {
    e.preventDefault()
    const file = fileRef.current?.files?.[0]
    if (!file) {
      setMsg('CHOOSE A cookies.txt FILE FIRST.')
      return
    }
    setBusy(true)
    setMsg('')
    try {
      const text = await file.text()
      await api.postText('/api/downloads/cookies', text)
      setMsg('COOKIES STORED. LOGIN-GATED DOWNLOADS SHOULD NOW WORK.')
      if (fileRef.current) fileRef.current.value = ''
      await refresh()
    } catch {
      setMsg('UPLOAD FAILED — IS IT A NETSCAPE cookies.txt?')
    }
    setBusy(false)
  }

  async function remove() {
    await api.del('/api/downloads/cookies')
    setMsg('COOKIES REMOVED.')
    await refresh()
  }

  return (
    <section className="section">
      <h2>🍪 LOGIN COOKIES (VIMEO / PRIVATE SITES)</h2>
      <p className="fineprint">
        Some videos (e.g. Vimeo) only download with a logged-in session. Export your cookies as
        a Netscape-format <code>cookies.txt</code> (a browser extension like “Get cookies.txt
        LOCALLY” does this), then upload it here. No server access needed.
      </p>
      <form className="dlform" onSubmit={upload}>
        <input ref={fileRef} type="file" accept=".txt,text/plain" />
        <button type="submit" disabled={busy}>
          {busy ? 'UPLOADING…' : '📤 UPLOAD COOKIES'}
        </button>
      </form>
      <div className="fineprint">
        STATUS:{' '}
        {status?.present ? (
          <span className="pill done">STORED · {fmtSize(status.size)}</span>
        ) : (
          <span className="pill failed">NONE</span>
        )}{' '}
        {status?.present ? (
          <button className="link danger" onClick={remove}>
            REMOVE
          </button>
        ) : null}
      </div>
      {msg ? <div className="notice">{msg}</div> : null}
    </section>
  )
}

/* ── Media (Sonarr / Radarr) ─────────────────────────────────────────────── */

function Media({ api }) {
  const [service, setService] = useState('sonarr')
  const [status, setStatus] = useState(null)
  const [queue, setQueue] = useState(null)
  const [calendar, setCalendar] = useState(null)
  const [term, setTerm] = useState('')
  const [results, setResults] = useState(null)
  const [msg, setMsg] = useState('')
  const reqId = useRef(0)

  const load = useCallback(
    async (what) => {
      const id = ++reqId.current
      try {
        if (what === 'status') {
          const v = await api.get(`/api/media/${service}/status`)
          if (id === reqId.current) setStatus(v)
        } else if (what === 'queue') {
          const v = await api.get(`/api/media/${service}/queue`)
          if (id === reqId.current) setQueue(v)
        } else if (what === 'calendar') {
          const v = await api.get(`/api/media/${service}/calendar`)
          if (id === reqId.current) setCalendar(v)
        }
      } catch {
        if (id === reqId.current) setMsg('COULD NOT REACH THE BUSINESS PARTNER.')
      }
    },
    [api, service],
  )

  useEffect(() => {
    setStatus(null)
    setQueue(null)
    setCalendar(null)
    setResults(null)
    setMsg('')
    load('status')
  }, [service, load])

  async function doSearch(e) {
    e.preventDefault()
    if (!term.trim()) return
    setMsg('')
    try {
      const v = await api.get(
        `/api/media/${service}/search?term=${encodeURIComponent(term.trim())}`,
      )
      setResults(v?.results || [])
    } catch {
      setMsg('SEARCH FAILED.')
    }
  }

  async function add(title) {
    setMsg('')
    try {
      const r = await api.post(`/api/media/${service}/add`, { term: title })
      setMsg(
        r?.already
          ? `“${r.title}” is ALREADY in the vault.`
          : r?.added
            ? `ACQUIRED “${r.title}”! A trusted agent is fetching it.`
            : 'Could not complete the transaction.',
      )
      load('status')
    } catch {
      setMsg('ADD FAILED.')
    }
  }

  const emoji = service === 'radarr' ? '🎬' : '📺'

  return (
    <>
      <div className="tabs sub">
        {['sonarr', 'radarr'].map((s) => (
          <button
            key={s}
            className={'tab ' + (service === s ? 'active' : '')}
            onClick={() => setService(s)}
          >
            {s === 'radarr' ? '🎬 RADARR (MOVIES)' : '📺 SONARR (TV)'}
          </button>
        ))}
      </div>

      <div className="grid">
        <Stat label="BUSINESS PARTNER" value={`${emoji} ${service.toUpperCase()}`} />
        <Stat label="VAULT VERSION" value={status?.version ?? '—'} />
        <Stat label="LOTS OWNED" value={status?.library ?? '—'} />
        <Stat label="IN TRANSIT" value={status?.downloading ?? '—'} />
      </div>

      <section className="section">
        <h2>{emoji} SEARCH THE MARKET</h2>
        <form className="dlform" onSubmit={doSearch}>
          <input
            type="text"
            placeholder={`TYPE A ${service === 'radarr' ? 'MOVIE' : 'TV SHOW'} TITLE`}
            value={term}
            onChange={(e) => setTerm(e.target.value)}
          />
          <button type="submit">🔎 FIND</button>
        </form>
        {msg ? <div className="notice">{msg}</div> : null}
        {results && (
          <div className="cards">
            {results.length ? (
              results.map((r, i) => (
                <div className="mcard" key={i}>
                  {r.poster ? (
                    <img src={r.poster} alt="" loading="lazy" />
                  ) : (
                    <div className="poster-ph">NO PICTURE</div>
                  )}
                  <div className="mbody">
                    <div className="mtitle">
                      {r.title} {r.year ? `(${r.year})` : ''}
                    </div>
                    <div className="muted small clamp">{r.overview || 'No details.'}</div>
                    {r.in_library ? (
                      <span className="pill done">IN VAULT</span>
                    ) : (
                      <button onClick={() => add(r.title)}>🤑 ACQUIRE</button>
                    )}
                  </div>
                </div>
              ))
            ) : (
              <div className="muted">No results, my friend.</div>
            )}
          </div>
        )}
      </section>

      <div className="two-col">
        <section className="section">
          <div className="rowhead">
            <h2>📦 IN TRANSIT (QUEUE)</h2>
            <button className="mini" onClick={() => load('queue')}>
              REFRESH
            </button>
          </div>
          {queue?.records?.length ? (
            queue.records.map((r, i) => (
              <div className="qrow" key={i}>
                <div className="qtitle">{r.title}</div>
                <div className="qmeta">
                  {r.status} {r.timeleft ? `· ${r.timeleft}` : ''}
                </div>
                <div className="bar">
                  <span style={{ width: Math.max(2, r.progress || 0) + '%' }} />
                </div>
              </div>
            ))
          ) : (
            <div className="muted">Nothing in transit.</div>
          )}
        </section>

        <section className="section">
          <div className="rowhead">
            <h2>📅 ARRIVING SOON</h2>
            <button className="mini" onClick={() => load('calendar')}>
              REFRESH
            </button>
          </div>
          {calendar?.items?.length ? (
            calendar.items.map((e, i) => (
              <div className="calrow" key={i}>
                <span className="caldate">{e.date}</span>
                <span>
                  {e.series ? <b>{e.series}</b> : ''}{' '}
                  {e.season != null ? `S${pad(e.season)}E${pad(e.episode)}` : ''} {e.title}
                </span>
              </div>
            ))
          ) : (
            <div className="muted">Nothing arriving yet.</div>
          )}
        </section>
      </div>
    </>
  )
}

/* ── Schedules ───────────────────────────────────────────────────────────── */

function Schedules({ api }) {
  const [rows, setRows] = useState([])
  const [form, setForm] = useState({
    guild_id: '',
    channel_id: '',
    kind: 'message',
    message: '',
    interval_seconds: '',
    minutes: '1',
  })
  const [msg, setMsg] = useState('')

  const refresh = useCallback(async () => {
    try {
      const d = await api.get('/api/schedules')
      setRows(d?.schedules || [])
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, 15000)
    return () => clearInterval(t)
  }, [refresh])

  async function create(e) {
    e.preventDefault()
    setMsg('')
    const body = {
      guild_id: Number(form.guild_id),
      channel_id: Number(form.channel_id),
      kind: form.kind,
      message: form.message || null,
      interval_seconds: form.interval_seconds ? Number(form.interval_seconds) : null,
      minutes: form.minutes ? Number(form.minutes) : 1,
    }
    if (!body.guild_id || !body.channel_id) {
      setMsg('GUILD ID AND CHANNEL ID ARE COMPULSORY.')
      return
    }
    try {
      await api.post('/api/schedules', body)
      setMsg('SCHEDULE SEALED IN THE CONTRACT.')
      setForm({ ...form, message: '' })
      await refresh()
    } catch {
      setMsg('COULD NOT SEAL THE CONTRACT.')
    }
  }

  async function toggle(row) {
    await api.post(`/api/schedules/${row.id}/enabled`, { enabled: !row.enabled })
    await refresh()
  }

  async function remove(id) {
    await api.del('/api/schedules/' + id)
    await refresh()
  }

  return (
    <>
      <section className="section highlight">
        <h2>⏰ NEW PAYMENT PLAN (SCHEDULE)</h2>
        <form className="form-grid" onSubmit={create}>
          <label>
            GUILD ID
            <input
              value={form.guild_id}
              onChange={(e) => setForm({ ...form, guild_id: e.target.value })}
              placeholder="e.g. 123456789"
            />
          </label>
          <label>
            CHANNEL ID
            <input
              value={form.channel_id}
              onChange={(e) => setForm({ ...form, channel_id: e.target.value })}
              placeholder="e.g. 987654321"
            />
          </label>
          <label>
            KIND
            <select value={form.kind} onChange={(e) => setForm({ ...form, kind: e.target.value })}>
              <option value="message">MESSAGE</option>
              <option value="digest_sonarr">SONARR DIGEST</option>
              <option value="digest_radarr">RADARR DIGEST</option>
            </select>
          </label>
          <label>
            MESSAGE (OPTIONAL)
            <input
              value={form.message}
              onChange={(e) => setForm({ ...form, message: e.target.value })}
              placeholder="Kindly send your bank details…"
            />
          </label>
          <label>
            REPEAT EVERY (SECONDS, BLANK=ONCE)
            <input
              value={form.interval_seconds}
              onChange={(e) => setForm({ ...form, interval_seconds: e.target.value })}
              placeholder="3600"
            />
          </label>
          <label>
            FIRST RUN IN (MINUTES)
            <input
              value={form.minutes}
              onChange={(e) => setForm({ ...form, minutes: e.target.value })}
              placeholder="1"
            />
          </label>
          <button type="submit">🖋 SEAL CONTRACT</button>
        </form>
        {msg ? <div className="notice">{msg}</div> : null}
      </section>

      <section className="section">
        <h2>📜 ACTIVE CONTRACTS</h2>
        <table>
          <thead>
            <tr>
              <th>REF</th>
              <th>KIND</th>
              <th>GUILD / CHANNEL</th>
              <th>CADENCE</th>
              <th>NEXT</th>
              <th>STATE</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.length ? (
              rows.map((r) => (
                <tr key={r.id} className={r.enabled ? '' : 'disabled-row'}>
                  <td>#{r.id}</td>
                  <td>{r.kind}</td>
                  <td className="small">
                    G:{r.guild_id}
                    <br />
                    C:{r.channel_id}
                  </td>
                  <td>{r.interval_seconds ? r.interval_seconds + 's' : 'once'}</td>
                  <td>{new Date(r.next_run_at).toLocaleString()}</td>
                  <td>
                    <span className={'pill ' + (r.enabled ? 'done' : 'queued')}>
                      {r.enabled ? 'ACTIVE' : 'PAUSED'}
                    </span>
                  </td>
                  <td className="row-actions">
                    <button className="link" onClick={() => toggle(r)}>
                      {r.enabled ? 'PAUSE' : 'RESUME'}
                    </button>
                    <button className="link danger" onClick={() => remove(r.id)}>
                      DESTROY
                    </button>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="7" className="muted">
                  No active contracts.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
    </>
  )
}

/* ── Users ───────────────────────────────────────────────────────────────── */

function Users({ api }) {
  const [users, setUsers] = useState([])

  const refresh = useCallback(async () => {
    try {
      const d = await api.get('/api/users')
      setUsers(d?.users || [])
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function setRole(id, role) {
    await api.post(`/api/users/${id}/role`, { role })
    await refresh()
  }

  return (
    <section className="section">
      <h2>👑 REGISTER OF HONOURABLES (DISCORD)</h2>
      <p className="fineprint">
        The server owner and any <code>ADMIN_DISCORD_IDS</code> are always ADMIN. Changing a role
        here updates the vault immediately.
      </p>
      <table>
        <thead>
          <tr>
            <th>DISCORD ID</th>
            <th>NAME</th>
            <th>ROLE</th>
            <th>JOINED</th>
          </tr>
        </thead>
        <tbody>
          {users.length ? (
            users.map((u) => (
              <tr key={u.discord_id}>
                <td>
                  <code>{u.discord_id}</code>
                </td>
                <td>{u.discord_name || '—'}</td>
                <td>
                  <RoleSelect value={u.role} onChange={(r) => setRole(u.discord_id, r)} />
                </td>
                <td className="small">{new Date(u.created_at).toLocaleDateString()}</td>
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan="4" className="muted">
                No honourables on record yet.
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </section>
  )
}

/* ── Telegram ────────────────────────────────────────────────────────────── */

function Telegram({ api }) {
  const [status, setStatus] = useState(null)
  const [users, setUsers] = useState([])

  const refresh = useCallback(async () => {
    try {
      const [s, u] = await Promise.all([
        api.get('/api/telegram/status'),
        api.get('/api/telegram/users'),
      ])
      setStatus(s)
      setUsers(u?.users || [])
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function setRole(id, role) {
    await api.post(`/api/telegram/users/${id}/role`, { role })
    await refresh()
  }

  return (
    <>
      <div className="grid">
        <Stat
          label="CABLEGRAM SERVICE"
          value={
            <span className={'pill ' + (status?.configured ? 'done' : 'failed')}>
              {status?.configured ? 'ONLINE' : 'NOT SET'}
            </span>
          }
        />
        <Stat label="NOTIFY CHAT ID" value={status?.chat_id || '—'} />
        <Stat label="SUBSCRIBERS" value={status?.users ?? '—'} />
      </div>

      <section className="section highlight">
        <h2>✉️ CABLEGRAM OFFICE (TELEGRAM BOT)</h2>
        <p className="letter">
          Your Telegram agent can be reached by cablegram on the bot. It accepts
          <code> /download</code> <code> /downloads</code> <code> /status</code> and
          <code> /whoami</code>. Finished cargo is announced automatically. To enable, set
          <code> TELEGRAM_BOT_TOKEN</code> (and <code>TELEGRAM_CHAT_ID</code>) in the stack and make
          the <code>nigerianbot-telegram</code> image public ONCE.
        </p>
      </section>

      <section className="section">
        <h2>📇 TELEGRAM SUBSCRIBERS</h2>
        <table>
          <thead>
            <tr>
              <th>TELEGRAM ID</th>
              <th>USERNAME</th>
              <th>ROLE</th>
              <th>JOINED</th>
            </tr>
          </thead>
          <tbody>
            {users.length ? (
              users.map((u) => (
                <tr key={u.telegram_id}>
                  <td>
                    <code>{u.telegram_id}</code>
                  </td>
                  <td>{u.username ? '@' + u.username : '—'}</td>
                  <td>
                    <RoleSelect value={u.role} onChange={(r) => setRole(u.telegram_id, r)} />
                  </td>
                  <td className="small">{new Date(u.created_at).toLocaleDateString()}</td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="4" className="muted">
                  No subscribers yet. Message the bot to register.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
    </>
  )
}

/* ── Setup ───────────────────────────────────────────────────────────────── */

const SETUP = [
  {
    key: 'database',
    icon: '🗄️',
    title: 'POSTGRESQL (THE VAULT)',
    blurb: 'One database backs every surface: audit log, downloads, schedules, users.',
    env: [
      ['POSTGRES_USER', 'DB username (stack default: nigerian).'],
      ['POSTGRES_PASSWORD', 'Strong password — required.'],
      ['POSTGRES_DB', 'DB name (stack default: nigerian_bot).'],
      ['DATABASE_URL', 'Assembled by the stack; set manually for local dev.'],
    ],
    steps: [
      'In Portainer, set POSTGRES_PASSWORD before deploying.',
      'Every service runs migrations automatically on startup.',
    ],
    usage: ['Migrations in migrations/*.sql', 'Inspect via the Users / Downloads tabs'],
  },
  {
    key: 'api',
    icon: '🖥️',
    title: 'DASHBOARD / API',
    blurb:
      'This control panel and its REST API. Sign in with a PIN (default 1234, changed on first login).',
    env: [
      ['DASHBOARD_PIN', 'PIN seeded on first run (default 1234); changed in the UI.'],
      ['API_KEY', 'Optional. Alternative credential for scripts/clients.'],
      ['JWT_SECRET', 'Signs dashboard tokens — long & random.'],
      ['API_PORT', 'Host/container port (default 8000).'],
      ['PUBLIC_BASE_URL', 'Optional public URL, used in notification links.'],
    ],
    steps: [
      'Set JWT_SECRET in the stack (DASHBOARD_PIN optional).',
      'Open http://<server>:8000/ and sign in with PIN 1234.',
      'Choose a new PIN when prompted; change it anytime below.',
    ],
    usage: ['Every tab in this dashboard', 'REST: GET /api/*, POST /api/*'],
  },
  {
    key: 'discord',
    icon: '🤖',
    title: 'DISCORD BOT',
    blurb: 'The primary surface: slash commands, music, voice pool, scheduler, audit log.',
    env: [
      ['DISCORD_TOKEN', 'Bot token (Developer Portal → Bot → Reset Token).'],
      ['DISCORD_GUILD_ID', 'Your server id → instant command registration.'],
      ['DISCORD_TOKEN_2..9', 'Optional extra bots for simultaneous voice.'],
      ['ADMIN_DISCORD_IDS', 'Comma-separated ids always treated as Admin.'],
    ],
    steps: [
      'Create an app at discord.com/developers/applications.',
      'Bot → copy the token into DISCORD_TOKEN.',
      'OAuth2 → URL Generator: scopes "bot" + "applications.commands", invite it.',
      'Set DISCORD_GUILD_ID for instant commands; give extra bots Connect/Speak.',
    ],
    usage: ['/music play <song|url>', '/download <url>', '/sonarr · /radarr', '/schedule · /admin'],
  },
  {
    key: 'sonarr',
    icon: '📺',
    title: 'SONARR (TV)',
    blurb: 'Browse, search, queue, and request TV series from Discord or the Media tab.',
    env: [
      ['SONARR_URL', 'e.g. http://192.168.1.10:8989 (reachable from containers).'],
      ['SONARR_API_KEY', 'Sonarr → Settings → General → API Key.'],
      ['SONARR_IMPORT_PATH', 'How Sonarr sees the shared downloads folder (for Import).'],
    ],
    steps: [
      'Use the LAN IP (not localhost) so containers can reach it.',
      'Paste URL + API key into the stack; redeploy.',
      'Test from the Media tab → SONARR → search.',
    ],
    usage: ['Media tab → Sonarr', '/sonarr status · queue · upcoming · search · add'],
  },
  {
    key: 'radarr',
    icon: '🎬',
    title: 'RADARR (MOVIES)',
    blurb: 'The same flow for movies.',
    env: [
      ['RADARR_URL', 'e.g. http://192.168.1.10:7878 (reachable from containers).'],
      ['RADARR_API_KEY', 'Radarr → Settings → General → API Key.'],
      ['RADARR_IMPORT_PATH', 'How Radarr sees the shared downloads folder (for Import).'],
    ],
    steps: [
      'Use the LAN IP so the API container can reach it.',
      'Paste URL + API key into the stack; redeploy.',
      'Test from the Media tab → RADARR → search.',
    ],
    usage: ['Media tab → Radarr', '/radarr status · queue · upcoming · search · add'],
  },
  {
    key: 'music',
    icon: '🎵',
    title: 'MUSIC LIBRARY',
    blurb: 'Play local files or stream URLs into Discord voice channels.',
    env: [
      ['MUSIC_HOST_PATH', 'Folder on the server to mount (read-only).'],
      ['MUSIC_MOUNT_PATH', 'Where it appears in the container (default /music).'],
    ],
    steps: [
      'Point MUSIC_HOST_PATH at your music folder and redeploy the bot.',
      'Files are decoded via ffmpeg, so most formats work.',
      'yt-dlp streams YouTube/Vimeo URLs directly.',
    ],
    usage: ['/music play (autocompletes local files)', '/music search · queue · volume'],
  },
  {
    key: 'downloads',
    icon: '📥',
    title: 'DOWNLOADS (WORKER)',
    blurb: 'Any surface queues a URL; the worker fetches it to server storage.',
    env: [
      ['DOWNLOADS_PATH', 'Container path for finished files (served at /media).'],
      ['DISCORD_NOTIFY_WEBHOOK', 'Optional Discord webhook for "done" pings.'],
      ['YTDLP_COOKIES_FILE', 'Stored cookies path (advanced; default /cookies/cookies.txt).'],
    ],
    steps: [
      'Set DOWNLOADS_HOST_PATH to a host folder Sonarr/Radarr also mount (chown 10001:10001).',
      'Queue a URL from the Downloads tab (CARGO); add a "Save as" name for clean matching.',
      'For Vimeo/login-gated videos, upload cookies.txt in the Downloads tab.',
      'Click 📥 IMPORT on a finished item to send it to Sonarr/Radarr.',
    ],
    usage: ['Downloads tab', 'Discord /download <url>', 'Telegram /download <url>'],
  },
  {
    key: 'telegram',
    icon: '✉️',
    title: 'TELEGRAM BOT',
    blurb: 'A second surface over the same core: commands + download notifications.',
    env: [
      ['TELEGRAM_BOT_TOKEN', 'From @BotFather (required to run the service).'],
      ['TELEGRAM_CHAT_ID', 'Chat that gets "download finished" notices.'],
      ['TELEGRAM_ADMIN_IDS', 'Comma-separated ids always treated as Admin.'],
    ],
    steps: [
      'Message @BotFather → /newbot → copy the token.',
      'Send any message to your bot, then read the chat id from getUpdates.',
      'Set both in the stack and redeploy; make the telegram image public once.',
    ],
    usage: ['/start · /help · /status', '/download <url> · /downloads · /whoami'],
  },
  {
    key: 'deploy',
    icon: '🚀',
    title: 'DEPLOY & CI',
    blurb: 'Push to main; GitHub Actions builds images to GHCR; Portainer pulls them.',
    env: [
      ['GHCR', 'ghcr.io/smartcile/nigerianbot-<svc>:latest (bot, api, worker, telegram).'],
      ['Portainer', 'Deploy docker-compose.yml as a Stack; set env vars there.'],
    ],
    steps: [
      'git push to main triggers .github/workflows/build.yml.',
      'Make each new GHCR package public ONCE (worker, telegram).',
      'In Portainer: Stacks → Pull and redeploy.',
    ],
    usage: ['Never use compose "build:" in Portainer', 'Use registry cache, not type=gha'],
  },
]

function statusFor(key, st) {
  if (!st) return null
  switch (key) {
    case 'database':
      return st.database ? { ok: true, text: 'CONNECTED' } : { ok: false, text: 'DOWN' }
    case 'api':
      return st.pin_default
        ? { ok: false, text: 'DEFAULT PIN!' }
        : { ok: true, text: 'ONLINE' }
    case 'discord':
      return st.bot?.configured
        ? { ok: true, text: `ACTIVE · ${st.bot.commands} CMD` }
        : { ok: false, text: 'AWAITING BOT' }
    case 'sonarr':
      return st.sonarr?.configured
        ? { ok: true, text: st.sonarr.url || 'CONFIGURED' }
        : { ok: false, text: 'NOT SET' }
    case 'radarr':
      return st.radarr?.configured
        ? { ok: true, text: st.radarr.url || 'CONFIGURED' }
        : { ok: false, text: 'NOT SET' }
    case 'telegram':
      return st.telegram?.configured
        ? { ok: true, text: st.telegram.chat_id ? 'CHAT ' + st.telegram.chat_id : 'TOKEN SET' }
        : { ok: false, text: 'NOT SET' }
    case 'downloads':
      return st.downloads
        ? { ok: true, text: `${st.downloads.done} DONE · ${st.downloads.active} ACTIVE` }
        : null
    case 'music':
      return { ok: null, text: 'CHECK MOUNT' }
    case 'deploy':
      return { ok: null, text: 'SEE STEPS' }
    default:
      return null
  }
}

function Setup({ api }) {
  const [st, setSt] = useState(null)
  const [updated, setUpdated] = useState('')

  const refresh = useCallback(async () => {
    try {
      setSt(await api.get('/api/setup/status'))
      setUpdated(new Date().toLocaleTimeString())
    } catch {
      /* handled */
    }
  }, [api])

  useEffect(() => {
    refresh()
  }, [refresh])

  return (
    <>
      <section className="section highlight">
        <h2>📖 HOW TO CONNECT EVERYTHING</h2>
        <p className="letter">
          Live status below is read from the running services (secrets are never shown — only
          whether they are set). Set these variables in your Portainer stack, then
          <b> Pull and redeploy</b>. Full networking notes: <code>docs/DEPLOYMENT.md</code>.
        </p>
        <div className="setup-summary">
          <span>
            🔗 PUBLIC BASE URL: <b>{st?.public_base_url || 'not set'}</b>
          </span>
          <button className="mini" onClick={refresh}>
            REFRESH STATUS
          </button>
        </div>
      </section>

      <div className="setup-grid">
        {SETUP.map((s) => {
          const stat = statusFor(s.key, st)
          return (
            <article className="setup-card" key={s.key}>
              <header>
                <span className="setup-icon">{s.icon}</span>
                <h3>{s.title}</h3>
                {stat && (
                  <span className={'pill ' + (stat.ok === true ? 'done' : stat.ok === false ? 'failed' : 'queued')}>
                    {stat.text}
                  </span>
                )}
              </header>
              <p className="muted small">{s.blurb}</p>

              <h4>ENVIRONMENT</h4>
              <table className="env">
                <tbody>
                  {s.env.map(([k, hint]) => (
                    <tr key={k}>
                      <td>
                        <code>{k}</code>
                      </td>
                      <td className="muted small">{hint}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <h4>HOW TO CONNECT</h4>
              <ol className="steps">
                {s.steps.map((step, i) => (
                  <li key={i}>{step}</li>
                ))}
              </ol>

              <h4>HOW TO USE</h4>
              <div className="chips">
                {s.usage.map((u) => (
                  <span className="chip" key={u}>
                    {u}
                  </span>
                ))}
              </div>
            </article>
          )
        })}
      </div>

      <ChangePinCard api={api} />

      <div className="updated">STATUS SYNCED {updated}</div>
    </>
  )
}

function ChangePinCard({ api }) {
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirm, setConfirm] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)

  async function submit(e) {
    e.preventDefault()
    setMsg('')
    if (next !== confirm) {
      setMsg('THE TWO PINS DO NOT MATCH.')
      return
    }
    setBusy(true)
    try {
      await api.post('/api/auth/change-pin', { current_pin: current, new_pin: next })
      setMsg('PIN UPDATED. KEEP IT SECRET, MY FRIEND.')
      setCurrent('')
      setNext('')
      setConfirm('')
    } catch {
      setMsg('CURRENT PIN WRONG, OR NEW PIN NOT 4-8 DIGITS.')
    }
    setBusy(false)
  }

  return (
    <section className="section highlight">
      <h2>🔐 CHANGE DASHBOARD PIN</h2>
      <form className="form-grid" onSubmit={submit}>
        <label>
          CURRENT PIN
          <input
            type="password"
            inputMode="numeric"
            maxLength={8}
            value={current}
            onChange={(e) => setCurrent(e.target.value)}
          />
        </label>
        <label>
          NEW PIN (4-8 DIGITS)
          <input
            type="password"
            inputMode="numeric"
            maxLength={8}
            value={next}
            onChange={(e) => setNext(e.target.value)}
          />
        </label>
        <label>
          CONFIRM NEW PIN
          <input
            type="password"
            inputMode="numeric"
            maxLength={8}
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
          />
        </label>
        <button type="submit" disabled={busy}>
          {busy ? 'SAVING…' : 'SAVE NEW PIN'}
        </button>
      </form>
      {msg ? <div className="notice">{msg}</div> : null}
    </section>
  )
}

/* ── Bits ────────────────────────────────────────────────────────────────── */

function RoleSelect({ value, onChange }) {
  return (
    <select className="role" value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="admin">ADMIN</option>
      <option value="user">USER</option>
      <option value="viewer">VIEWER</option>
    </select>
  )
}

function Stat({ label, value }) {
  return (
    <div className="stat">
      <div className="label">{label}</div>
      <div className="value">{value}</div>
    </div>
  )
}

function Ticker() {
  const text =
    'URGENT BUSINESS PROPOSAL ·· YOU HAVE BEEN SELECTED TO RECEIVE US$45,000,000.00 USD ·· ' +
    'KINDLY SEND YOUR BANK PARTICULARS ·· 100% GUARANTEED ·· GOD BLESS ·· NO RISK INVOLVED ·· '
  return (
    <div className="ticker">
      <div className="ticker-track">🇳🇬 {text.repeat(2)}</div>
    </div>
  )
}

/* A gold "official" seal. Pure SVG — swap in an AI image if you like. */
function Seal() {
  return (
    <svg viewBox="0 0 120 120" className="seal" role="img" aria-label="Official seal">
      <defs>
        <radialGradient id="g" cx="50%" cy="40%" r="70%">
          <stop offset="0%" stopColor="#fff3b0" />
          <stop offset="55%" stopColor="#ffd700" />
          <stop offset="100%" stopColor="#b8860b" />
        </radialGradient>
      </defs>
      <circle cx="60" cy="60" r="56" fill="url(#g)" stroke="#7a5c00" strokeWidth="4" />
      <circle cx="60" cy="60" r="44" fill="none" stroke="#7a5c00" strokeWidth="2" strokeDasharray="6 4" />
      <text x="60" y="52" textAnchor="middle" fontSize="26">
        🇳🇬
      </text>
      <text x="60" y="72" textAnchor="middle" fontSize="11" fontWeight="bold" fill="#5b4300">
        CERTIFIED
      </text>
      <text x="60" y="85" textAnchor="middle" fontSize="9" fill="#5b4300">
        US$45,000,000
      </text>
    </svg>
  )
}

/* AI-image placeholder: a framed portrait slot. Replace `src` with a real image
   URL (e.g. an AI-generated "Chief Auditor") to use it. */
function AiImage({ label = 'HONOURABLE', caption = '', src = '' }) {
  return (
    <aside className="ai-frame">
      <div className="ai-inner">
        {src ? (
          <img src={src} alt={label} />
        ) : (
          <svg viewBox="0 0 200 220" role="img" aria-label={label}>
            <rect width="200" height="220" fill="#0b3d1f" />
            <circle cx="100" cy="80" r="42" fill="#ffd700" opacity="0.85" />
            <path d="M20 220c0-48 36-72 80-72s80 24 80 72z" fill="#ffd700" opacity="0.85" />
            <text x="100" y="205" textAnchor="middle" fontSize="14" fontWeight="bold" fill="#063">
              🇳🇬
            </text>
          </svg>
        )}
      </div>
      <div className="ai-label">{label}</div>
      <div className="ai-caption">{caption}</div>
    </aside>
  )
}

function pad(n) {
  return String(n ?? 0).padStart(2, '0')
}

function fmtSize(bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = bytes
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return i === 0 ? bytes + ' B' : v.toFixed(1) + ' ' + units[i]
}
