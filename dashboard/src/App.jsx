import { useState, useEffect, useLayoutEffect, useCallback, useMemo, useRef } from 'react'
import { CopyContext, copyFor, useCopy } from './copy'

const TOKEN_KEY = 'nb_token'
const CHANGE_KEY = 'nb_must_change'
const THEME_KEY = 'nb_theme'

export default function App() {
  const [token, setToken] = useState(() => localStorage.getItem(TOKEN_KEY))
  const [mustChange, setMustChange] = useState(() => localStorage.getItem(CHANGE_KEY) === '1')
  const [theme, setTheme] = useState(() =>
    localStorage.getItem(THEME_KEY) === 'classic' ? 'classic' : '419',
  )

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme
    localStorage.setItem(THEME_KEY, theme)
  }, [theme])

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

  const copy = useMemo(() => copyFor(theme), [theme])

  let screen
  if (!token) screen = <Login onLogin={login} />
  else if (mustChange) screen = <ForceChangePin token={token} onDone={pinChanged} onLogout={logout} />
  else screen = <Shell token={token} onLogout={logout} theme={theme} onTheme={setTheme} />

  return <CopyContext.Provider value={copy}>{screen}</CopyContext.Provider>
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
    async put(path, body) {
      return json(
        check(
          await fetch(path, {
            method: 'PUT',
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
  const copy = useCopy()
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
        setErr(copy.loginErrorPin)
        return
      }
      if (!r.ok) {
        setErr(copy.loginErrorFailed)
        return
      }
      const data = await r.json()
      onLogin(data.token, data.must_change_pin)
    } catch {
      setErr(copy.loginErrorServer)
    }
  }

  return (
    <div className="login">
      <div className="popup-note">{copy.loginBadge}</div>
      <form className="card" onSubmit={submit}>
        <div className="seal-wrap">
          <Seal />
        </div>
        <h1>{copy.loginTitle}</h1>
        <p className="scam-sub">{copy.loginSub}</p>
        <p className="letter">{copy.loginLetter}</p>
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder={copy.loginPlaceholder}
          value={pin}
          onChange={(e) => setPin(e.target.value)}
        />
        <button type="submit">{copy.loginButton}</button>
        <div className="err">{err}</div>
        <div className="fineprint">{copy.loginFineprint}</div>
      </form>
    </div>
  )
}

/* Forced PIN change on first sign-in. */
function ForceChangePin({ token, onDone, onLogout }) {
  const copy = useCopy()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirm, setConfirm] = useState('')
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)

  async function submit(e) {
    e.preventDefault()
    setErr('')
    if (next !== confirm) {
      setErr(copy.errPinMismatch)
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
        setErr(copy.errCurrentPin)
        setBusy(false)
        return
      }
      if (!r.ok) {
        setErr(copy.errWeakPin)
        setBusy(false)
        return
      }
      onDone()
    } catch {
      setErr(copy.errServer)
      setBusy(false)
    }
  }

  return (
    <div className="login">
      <div className="popup-note">{copy.changeBadge}</div>
      <form className="card" onSubmit={submit}>
        <div className="seal-wrap">
          <Seal />
        </div>
        <h1>{copy.changeTitle}</h1>
        <p className="scam-sub">{copy.changeSub}</p>
        <p className="letter">{copy.changeLetter}</p>
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="Current PIN (1234)"
          value={current}
          onChange={(e) => setCurrent(e.target.value)}
        />
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="New PIN (4-8 digits)"
          value={next}
          onChange={(e) => setNext(e.target.value)}
        />
        <input
          type="password"
          inputMode="numeric"
          maxLength={8}
          placeholder="Confirm new PIN"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
        />
        <button type="submit" disabled={busy}>
          {busy ? copy.saving : copy.savePin}
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

function Shell({ token, onLogout, theme, onTheme }) {
  const copy = useCopy()
  const [tab, setTab] = useState('overview')
  const [popup, setPopup] = useState(true)
  const api = useMemo(() => makeApi(token, onLogout), [token, onLogout])
  const tabs = [
    ['overview', copy.tabOverview],
    ['downloads', copy.tabDownloads],
    ['media', copy.tabMedia],
    ['schedules', copy.tabSchedules],
    ['compose', copy.tabCompose],
    ['users', copy.tabUsers],
    ['telegram', copy.tabTelegram],
    ['setup', copy.tabSetup],
  ]

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
            {copy.brand}
            <small>{copy.brandSub}</small>
          </span>
        </div>
        <div className="conf">★ CONFIDENTIAL ★</div>
        <button className="logout" onClick={onLogout}>
          {copy.logout}
        </button>
      </header>

      <nav className="tabs">
        {tabs.map(([id, label]) => (
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
        {tab === 'compose' && <Compose api={api} />}
        {tab === 'users' && <Users api={api} />}
        {tab === 'telegram' && <Telegram api={api} />}
        {tab === 'setup' && <Setup api={api} theme={theme} onTheme={onTheme} />}
      </main>

      <footer className="foot">{copy.footer}</footer>
    </>
  )
}

/* ── Overview ────────────────────────────────────────────────────────────── */

function Overview({ api }) {
  const copy = useCopy()
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
        <Stat label={copy.totalCommands} value={stats?.total ?? '—'} />
        <Stat label={copy.last24h} value={stats?.last_24h ?? '—'} />
        <Stat
          label={copy.database}
          value={
            <span className={'pill ' + (dbOk ? 'done' : 'failed')}>
              {dbOk ? copy.dbConnected : copy.dbDown}
            </span>
          }
        />
        <Stat
          label={copy.commandsLogged}
          value={copy.isClassic ? status?.commands_logged ?? '—' : '$45,000,000'}
        />
      </div>

      <div className="two-col">
        <section className="section">
          <h2>{copy.topCommands}</h2>
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
            <div className="muted">{copy.noCommands}</div>
          )}
        </section>

        <AiImage label="CHIEF AUDITOR" caption="AI IMAGE SLOT — replace src" />
      </div>

      <section className="section">
        <h2>{copy.recentActivity}</h2>
        <table>
          <thead>
            <tr>
              <th>TIME</th>
              <th>{copy.thUser}</th>
              <th>{copy.thCommand}</th>
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
                  {copy.noActivity}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>

      <div className="updated">
        {copy.autoRefresh} {updated}
      </div>
    </>
  )
}

/* ── Downloads ───────────────────────────────────────────────────────────── */

function Downloads({ api }) {
  const copy = useCopy()
  const [items, setItems] = useState([])
  const [url, setUrl] = useState('')
  const [saveAs, setSaveAs] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState('')
  const [importFile, setImportFile] = useState(null)
  const [bulk, setBulk] = useState('')
  const [bulkBusy, setBulkBusy] = useState(false)
  const [bulkMsg, setBulkMsg] = useState('')

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
      setNote(row?.id ? copy.queuedNote(row.id) : copy.submitted)
      setUrl('')
      setSaveAs('')
      await refresh()
    } catch {
      setNote(copy.transactionFailed)
    }
    setBusy(false)
  }

  async function remove(id) {
    await api.del('/api/downloads/' + id)
    await refresh()
  }

  async function submitBulk(e) {
    e.preventDefault()
    if (!bulk.trim()) return
    setBulkBusy(true)
    setBulkMsg('')
    try {
      const r = await api.post('/api/downloads/bulk', { urls: bulk })
      setBulkMsg(copy.bulkResult(r?.added ?? 0, r?.skipped ?? 0))
      setBulk('')
      await refresh()
    } catch {
      setBulkMsg(copy.bulkFailed)
    }
    setBulkBusy(false)
  }

  return (
    <>
      <section className="section highlight">
        <h2>{copy.addDownload}</h2>
        <form className="dlform" onSubmit={submit}>
          <input
            type="text"
            placeholder={copy.urlPlaceholder}
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
            {busy ? copy.sending : copy.download}
          </button>
        </form>
        <div className="fineprint">
          {copy.workerNote}
          {note ? <b> {note}</b> : null}
        </div>
      </section>

      <section className="section highlight">
        <h2>{copy.bulkImport}</h2>
        <form onSubmit={submitBulk}>
          <textarea
            rows={4}
            placeholder={
              'Paste a Tubi/YouTube series link, or many URLs (one per line).\nhttps://tubitv.com/series/300007017/police-ten-7'
            }
            value={bulk}
            onChange={(e) => setBulk(e.target.value)}
          />
          <button type="submit" disabled={bulkBusy}>
            {bulkBusy ? copy.expanding : copy.expandQueue}
          </button>
        </form>
        <div className="fineprint">
          {copy.bulkNote}
          {bulkMsg ? <b> {bulkMsg}</b> : null}
        </div>
      </section>

      <CookiesCard api={api} />

      <section className="section">
        <h2>{copy.downloads}</h2>
        <table>
          <thead>
            <tr>
              <th>TIME</th>
              <th>{copy.thItem}</th>
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
                        {copy.open}
                      </a>
                    ) : null}
                    {d.status === 'done' && d.file_path ? (
                      <button className="link" onClick={() => setImportFile(d.file_path)}>
                        {copy.import}
                      </button>
                    ) : null}
                    <button className="link" onClick={() => remove(d.id)}>
                      {copy.destroy}
                    </button>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="5" className="muted">
                  {copy.noDownloads}
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
  const copy = useCopy()
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
        setMsg(copy.couldNotReachMedia)
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
      setMsg(copy.importStarted)
    } catch {
      setMsg(copy.importFailed)
    }
    setBusy(false)
  }

  async function importProvided() {
    if (!candidate) return
    const entry = entryFor(candidate)
    if (service === 'sonarr' && (!entry.seriesId || !entry.episodeIds.length)) {
      setMsg(copy.sonarrNoMatch)
      return
    }
    if (service === 'radarr' && !entry.movieId) {
      setMsg(copy.radarrNoMatch)
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
      setMsg(copy.searchFailed)
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
      setMsg(copy.pickMatch)
      return
    }
    const override =
      service === 'radarr'
        ? { movieId: chosen.id }
        : { seriesId: chosen.id, episodeIds: episodeId ? [Number(episodeId)] : [] }
    if (service === 'sonarr' && !override.episodeIds.length) {
      setMsg(copy.pickEpisode)
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

        {loading ? <div className="muted">{copy.loading}</div> : null}

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
              {copy.file} <code>{candidate.path || candidate.name}</code>
              {candidate.quality?.quality?.name ? (
                <>
                  {' '}
                  · {copy.quality} <b>{candidate.quality.quality.name}</b>
                </>
              ) : null}
            </div>

            {service === 'sonarr' ? (
              <div className="fineprint">
                {copy.matched}{' '}
                {candidate.series?.title ? <b>{candidate.series.title}</b> : '— none —'}{' '}
                {candidate.episodes?.length
                  ? (candidate.episodes || [])
                      .map((e) => `S${pad(e.seasonNumber)}E${pad(e.episodeNumber)}`)
                      .join(', ')
                  : null}
              </div>
            ) : (
              <div className="fineprint">
                {copy.matched}{' '}
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
              {copy.importUseMatch}
            </button>

            <hr />

            <div className="fineprint">
              {copy.linkDifferent(service === 'radarr' ? 'movie' : 'show / episode')}
            </div>
            <form className="dlform" onSubmit={searchLink}>
              <input
                value={linkTerm}
                onChange={(e) => setLinkTerm(e.target.value)}
                placeholder="Search your library"
              />
              <button type="submit">{copy.search}</button>
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
                <option value="">{copy.pickEpisodeOption}</option>
                {episodes.map((e) => (
                  <option key={e.id} value={e.id}>
                    S{pad(e.seasonNumber)}E{pad(e.episodeNumber)} — {e.title}
                  </option>
                ))}
              </select>
            ) : null}

            {chosen ? (
              <button onClick={importLinked} disabled={busy} style={{ marginTop: 8 }}>
                {copy.importInto(chosen.title)}
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
  const copy = useCopy()
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
      setMsg(copy.chooseCookies)
      return
    }
    setBusy(true)
    setMsg('')
    try {
      const text = await file.text()
      await api.postText('/api/downloads/cookies', text)
      setMsg(copy.cookiesStored)
      if (fileRef.current) fileRef.current.value = ''
      await refresh()
    } catch {
      setMsg(copy.cookiesUploadFailed)
    }
    setBusy(false)
  }

  async function remove() {
    await api.del('/api/downloads/cookies')
    setMsg(copy.cookiesRemoved)
    await refresh()
  }

  return (
    <section className="section">
      <h2>{copy.loginCookies}</h2>
      <p className="fineprint">
        Some videos (e.g. Vimeo) only download with a logged-in session. Export your cookies as
        a Netscape-format <code>cookies.txt</code> (a browser extension like “Get cookies.txt
        LOCALLY” does this), then upload it here. No server access needed.
      </p>
      <form className="dlform" onSubmit={upload}>
        <input ref={fileRef} type="file" accept=".txt,text/plain" />
        <button type="submit" disabled={busy}>
          {busy ? copy.uploading : copy.uploadCookies}
        </button>
      </form>
      <div className="fineprint">
        {copy.status}{' '}
        {status?.present ? (
          <span className="pill done">
            {copy.stored} · {fmtSize(status.size)}
          </span>
        ) : (
          <span className="pill failed">{copy.none}</span>
        )}{' '}
        {status?.present ? (
          <button className="link danger" onClick={remove}>
            {copy.remove}
          </button>
        ) : null}
      </div>
      {msg ? <div className="notice">{msg}</div> : null}
    </section>
  )
}

/* ── Media (Sonarr / Radarr) ─────────────────────────────────────────────── */

function Media({ api }) {
  const copy = useCopy()
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
        if (id === reqId.current) setMsg(copy.couldNotReachPartner)
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
      setMsg(copy.searchFailed)
    }
  }

  async function add(title) {
    setMsg('')
    try {
      const r = await api.post(`/api/media/${service}/add`, { term: title })
      setMsg(
        r?.already
          ? copy.alreadyInLibrary(r.title)
          : r?.added
            ? copy.requested(r.title)
            : copy.couldNotComplete,
      )
      load('status')
    } catch {
      setMsg(copy.addFailed)
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
        <Stat label={copy.service} value={`${emoji} ${service.toUpperCase()}`} />
        <Stat label={copy.version} value={status?.version ?? '—'} />
        <Stat label={copy.library} value={status?.library ?? '—'} />
        <Stat label={copy.downloading} value={status?.downloading ?? '—'} />
      </div>

      <section className="section">
        <h2>
          {emoji} {copy.searchLibrary}
        </h2>
        <form className="dlform" onSubmit={doSearch}>
          <input
            type="text"
            placeholder={copy.typeTitle(service === 'radarr' ? 'movie' : 'TV show')}
            value={term}
            onChange={(e) => setTerm(e.target.value)}
          />
          <button type="submit">{copy.find}</button>
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
                    <div className="poster-ph">{copy.noPoster}</div>
                  )}
                  <div className="mbody">
                    <div className="mtitle">
                      {r.title} {r.year ? `(${r.year})` : ''}
                    </div>
                    <div className="muted small clamp">{r.overview || 'No details.'}</div>
                    {r.in_library ? (
                      <span className="pill done">{copy.inLibrary}</span>
                    ) : (
                      <button onClick={() => add(r.title)}>{copy.request}</button>
                    )}
                  </div>
                </div>
              ))
            ) : (
              <div className="muted">{copy.noResults}</div>
            )}
          </div>
        )}
      </section>

      <div className="two-col">
        <section className="section">
          <div className="rowhead">
            <h2>{copy.downloadQueue}</h2>
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
            <div className="muted">{copy.nothingDownloading}</div>
          )}
        </section>

        <section className="section">
          <div className="rowhead">
            <h2>{copy.upcoming}</h2>
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
            <div className="muted">{copy.nothingUpcoming}</div>
          )}
        </section>
      </div>
    </>
  )
}

/* ── Schedules ───────────────────────────────────────────────────────────── */

function Schedules({ api }) {
  const copy = useCopy()
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
      setMsg(copy.scheduleRequired)
      return
    }
    try {
      await api.post('/api/schedules', body)
      setMsg(copy.scheduleCreated)
      setForm({ ...form, message: '' })
      await refresh()
    } catch {
      setMsg(copy.scheduleFailed)
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
        <h2>{copy.newSchedule}</h2>
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
              placeholder={copy.messagePlaceholder}
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
          <button type="submit">{copy.createSchedule}</button>
        </form>
        {msg ? <div className="notice">{msg}</div> : null}
      </section>

      <section className="section">
        <h2>{copy.activeSchedules}</h2>
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
                      {copy.destroy}
                    </button>
                  </td>
                </tr>
              ))
            ) : (
              <tr>
                <td colSpan="7" className="muted">
                  {copy.noSchedules}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </section>
    </>
  )
}

/* ── Compose builder ─────────────────────────────────────────────────────── */

function blankProject() {
  return { id: null, name: '', description: '', model: { services: [], networks: [], volumes: [] } }
}

function projectFromRow(row) {
  return {
    id: row.id,
    name: row.name,
    description: row.description || '',
    model: row.model || { services: [], networks: [], volumes: [] },
  }
}

function newService(name) {
  return {
    name,
    image: '',
    build: null,
    ports: [],
    volumes: [],
    environment: [],
    networks: [],
    depends_on: [],
    restart: 'unless-stopped',
    command: null,
  }
}

const toLines = (text) =>
  text
    .split('\n')
    .map((s) => s.trim())
    .filter(Boolean)
const fromLines = (list) => (list || []).join('\n')
const envToText = (env) => (env || []).map((e) => `${e.key}=${e.value ?? ''}`).join('\n')
const secretKeysOf = (env) => (env || []).filter((e) => e.secret).map((e) => e.key)

function textToEnv(text, previous) {
  const secrets = new Set(secretKeysOf(previous))
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const i = line.indexOf('=')
      const key = (i === -1 ? line : line.slice(0, i)).trim()
      if (!key) return null
      const secret = secrets.has(key)
      return { key, value: secret ? null : i === -1 ? '' : line.slice(i + 1), secret }
    })
    .filter(Boolean)
}

function applySecrets(env, text) {
  const secrets = new Set(
    text
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean),
  )
  return (env || []).map((e) =>
    secrets.has(e.key) ? { ...e, secret: true, value: null } : { ...e, secret: false },
  )
}

function Compose({ api }) {
  const [projects, setProjects] = useState([])
  const [current, setCurrent] = useState(null)
  const [defaults, setDefaults] = useState([])
  const [files, setFiles] = useState([])
  const [composeDir, setComposeDir] = useState('')
  const [yaml, setYaml] = useState('')
  const [previewErr, setPreviewErr] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)
  const [dragOver, setDragOver] = useState(-1)
  const [defForm, setDefForm] = useState({
    kind: 'bind_mount',
    name: '',
    source: '',
    target: '',
    mode: 'rw',
    keys: '',
  })

  const load = useCallback(async () => {
    try {
      const [p, d, f] = await Promise.all([
        api.get('/api/compose/projects'),
        api.get('/api/compose/defaults'),
        api.get('/api/compose/files'),
      ])
      setProjects(p?.projects || [])
      setDefaults(d?.defaults || [])
      setFiles(f?.files || [])
      setComposeDir(f?.dir || '')
    } catch {
      /* api helper handles auth */
    }
  }, [api])

  useEffect(() => {
    load()
  }, [load])

  useEffect(() => {
    if (current) return
    setCurrent(projects.length ? projectFromRow(projects[0]) : blankProject())
  }, [projects, current])

  useEffect(() => {
    if (!current) return
    const t = setTimeout(async () => {
      try {
        const d = await api.post('/api/compose/render', {
          name: current.name || 'project',
          model: current.model,
        })
        if (d?.error) {
          setPreviewErr(d.message || 'Project is not valid yet.')
          setYaml('')
        } else {
          setYaml(d?.yaml || '')
          setPreviewErr('')
        }
      } catch {
        setPreviewErr('Could not render the preview.')
      }
    }, 400)
    return () => clearTimeout(t)
  }, [api, current])

  function change(patch) {
    setCurrent((c) => (c ? { ...c, ...patch } : c))
  }

  function updateService(index, fn) {
    setCurrent((c) => {
      if (!c) return c
      return {
        ...c,
        model: {
          ...c.model,
          services: c.model.services.map((s, i) => (i === index ? fn(s) : s)),
        },
      }
    })
  }

  function addService() {
    setCurrent((c) => {
      if (!c) return c
      const used = new Set(c.model.services.map((s) => s.name))
      let n = c.model.services.length + 1
      while (used.has('service' + n)) n += 1
      return {
        ...c,
        model: { ...c.model, services: [...c.model.services, newService('service' + n)] },
      }
    })
  }

  function removeService(index) {
    setCurrent((c) =>
      c
        ? { ...c, model: { ...c.model, services: c.model.services.filter((_, i) => i !== index) } }
        : c,
    )
  }

  function startDrag(e, item) {
    e.dataTransfer.setData('application/json', JSON.stringify(item))
    e.dataTransfer.effectAllowed = 'copy'
  }

  function dropDefault(e, index) {
    e.preventDefault()
    setDragOver(-1)
    let item
    try {
      item = JSON.parse(e.dataTransfer.getData('application/json'))
    } catch {
      return
    }
    if (!item || !item.kind) return
    const def = item.definition || {}

    updateService(index, (s) => {
      const next = { ...s }
      if (item.kind === 'bind_mount') {
        if (def.source && def.target) {
          const mode = def.mode ? ':' + def.mode : ''
          next.volumes = [...new Set([...(s.volumes || []), `${def.source}:${def.target}${mode}`])]
        }
      } else if (item.kind === 'network') {
        const network = def.network || item.name
        if (network) next.networks = [...new Set([...(s.networks || []), network])]
      } else if (item.kind === 'env_file') {
        const keys = Array.isArray(def.keys) ? def.keys : []
        const wanted = new Set(keys)
        const have = new Set((s.environment || []).map((v) => v.key))
        const existing = (s.environment || []).map((v) =>
          wanted.has(v.key) ? { ...v, secret: true, value: null } : v,
        )
        const added = keys
          .filter((k) => k && !have.has(k))
          .map((k) => ({ key: k, value: null, secret: true }))
        next.environment = [...existing, ...added]
      }
      return next
    })
    setMsg(`Applied "${item.name}" to the service.`)
  }

  async function save() {
    if (!current || !current.name.trim()) {
      setMsg('Give the project a name first (lowercase, digits, - or _).')
      return null
    }
    setBusy(true)
    setMsg('')
    try {
      const body = {
        name: current.name.trim(),
        description: current.description || null,
        model: current.model,
      }
      const saved = current.id
        ? await api.put(`/api/compose/projects/${current.id}`, body)
        : await api.post('/api/compose/projects', body)
      if (saved?.error) {
        setMsg(saved.message || 'Could not save the project.')
        return null
      }
      setCurrent(projectFromRow(saved))
      await load()
      setMsg('Project saved.')
      return saved
    } catch {
      setMsg('Could not save the project.')
      return null
    } finally {
      setBusy(false)
    }
  }

  async function publish() {
    const saved = await save()
    if (!saved) return
    setBusy(true)
    try {
      const published = await api.post(`/api/compose/projects/${saved.id}/publish`, {})
      if (published?.error) {
        setMsg(published.message || 'Publish failed.')
        return
      }
      setCurrent(projectFromRow(published))
      await load()
      setMsg('Published to ' + (published.published_path || 'the compose folder') + '.')
    } catch {
      setMsg('Publish failed.')
    } finally {
      setBusy(false)
    }
  }

  async function removeProject() {
    if (!current?.id) return
    if (!window.confirm(`Delete the saved project "${current.name}"? The published YAML stays on disk.`)) {
      return
    }
    try {
      const id = current.id
      await api.del('/api/compose/projects/' + id)
      setProjects((ps) => ps.filter((p) => p.id !== id))
      setCurrent(blankProject())
      await load()
      setMsg('Project deleted.')
    } catch {
      setMsg('Could not delete the project.')
    }
  }

  async function addDefault(e) {
    e.preventDefault()
    setMsg('')
    const definition =
      defForm.kind === 'bind_mount'
        ? {
            source: defForm.source.trim(),
            target: defForm.target.trim(),
            mode: defForm.mode.trim() || 'rw',
          }
        : defForm.kind === 'network'
          ? { network: defForm.name.trim() }
          : {
              keys: defForm.keys
                .split(',')
                .map((s) => s.trim())
                .filter(Boolean),
            }
    try {
      const r = await api.post('/api/compose/defaults', {
        kind: defForm.kind,
        name: defForm.name.trim(),
        definition,
      })
      if (r?.error) {
        setMsg(r.message || 'Could not save the default.')
        return
      }
      setDefForm({ kind: defForm.kind, name: '', source: '', target: '', mode: 'rw', keys: '' })
      await load()
      setMsg('Palette default saved.')
    } catch {
      setMsg('Could not save the default.')
    }
  }

  async function removeDefault(id) {
    try {
      await api.del('/api/compose/defaults/' + id)
      await load()
    } catch {
      setMsg('Could not remove the default.')
    }
  }

  if (!current) {
    return (
      <section className="section">
        <h2>🧩 COMPOSE BUILDER</h2>
        <div className="muted">Loading…</div>
      </section>
    )
  }

  return (
    <>
      <section className="section highlight">
        <h2>🧩 DOCKER COMPOSE BUILDER</h2>
        <div className="compose-toolbar">
          <select
            value={current.id ?? ''}
            onChange={(e) => {
              const id = Number(e.target.value)
              const row = projects.find((p) => p.id === id)
              setCurrent(row ? projectFromRow(row) : blankProject())
            }}
          >
            <option value="">— NEW PROJECT —</option>
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <input
            value={current.name}
            onChange={(e) => change({ name: e.target.value })}
            placeholder="PROJECT NAME (lowercase-slug)"
          />
          <input
            value={current.description}
            onChange={(e) => change({ description: e.target.value })}
            placeholder="DESCRIPTION (OPTIONAL)"
          />
          <button type="button" onClick={save} disabled={busy}>
            💾 SAVE
          </button>
          <button type="button" onClick={publish} disabled={busy}>
            🚀 PUBLISH
          </button>
          {current.id ? (
            <button type="button" className="link danger" onClick={removeProject} disabled={busy}>
              DELETE
            </button>
          ) : null}
        </div>
        {msg ? <div className="notice">{msg}</div> : null}
      </section>

      <div className="compose-layout">
        <section className="section">
          <h2>🎨 SAVED DEFAULTS</h2>
          <div className="compose-palette">
            {defaults.length ? (
              defaults.map((d) => (
                <div
                  key={d.id}
                  className="chip-drag"
                  draggable
                  onDragStart={(e) => startDrag(e, d)}
                  title="Drag onto a service card"
                >
                  <div>
                    {d.name} <span className="small muted">({d.kind.replace('_', ' ')})</span>
                  </div>
                  <button type="button" className="link danger" onClick={() => removeDefault(d.id)}>
                    REMOVE
                  </button>
                </div>
              ))
            ) : (
              <div className="muted small">
                No defaults yet. Add bind mounts, networks, or env keys below.
              </div>
            )}
          </div>

          <form className="form-grid" onSubmit={addDefault} style={{ marginTop: 12 }}>
            <label>
              KIND
              <select
                value={defForm.kind}
                onChange={(e) => setDefForm({ ...defForm, kind: e.target.value })}
              >
                <option value="bind_mount">BIND MOUNT</option>
                <option value="network">NETWORK</option>
                <option value="env_file">ENV FILE KEYS</option>
              </select>
            </label>
            <label>
              NAME
              <input
                value={defForm.name}
                onChange={(e) => setDefForm({ ...defForm, name: e.target.value })}
                placeholder={defForm.kind === 'network' ? 'proxy' : 'Media bind'}
              />
            </label>
            {defForm.kind === 'bind_mount' ? (
              <>
                <label>
                  HOST PATH
                  <input
                    value={defForm.source}
                    onChange={(e) => setDefForm({ ...defForm, source: e.target.value })}
                    placeholder="/srv/media"
                  />
                </label>
                <label>
                  CONTAINER PATH
                  <input
                    value={defForm.target}
                    onChange={(e) => setDefForm({ ...defForm, target: e.target.value })}
                    placeholder="/media"
                  />
                </label>
                <label>
                  MODE
                  <select
                    value={defForm.mode}
                    onChange={(e) => setDefForm({ ...defForm, mode: e.target.value })}
                  >
                    <option value="rw">rw</option>
                    <option value="ro">ro</option>
                  </select>
                </label>
              </>
            ) : null}
            {defForm.kind === 'env_file' ? (
              <label>
                KEYS (COMMA SEPARATED)
                <input
                  value={defForm.keys}
                  onChange={(e) => setDefForm({ ...defForm, keys: e.target.value })}
                  placeholder="API_KEY, DB_PASSWORD"
                />
              </label>
            ) : null}
            <button type="submit">+ SAVE DEFAULT</button>
          </form>

          <h2 style={{ marginTop: 16 }}>📦 PUBLISHED FILES</h2>
          <div className="muted small">{composeDir || 'COMPOSE_DIR'}</div>
          <div className="compose-palette" style={{ marginTop: 6 }}>
            {files.length ? (
              files.map((f) => (
                <div key={f.name} className="published-file">
                  <b>{f.name}</b>
                  <span className="small muted">{Math.max(1, Math.round(f.size / 1024))} KB</span>
                </div>
              ))
            ) : (
              <div className="muted small">Nothing published yet.</div>
            )}
          </div>
        </section>

        <section className="section">
          <div className="rowhead">
            <h2>🧱 SERVICES</h2>
            <button type="button" className="mini" onClick={addService}>
              + ADD SERVICE
            </button>
          </div>
          {current.model.services.length ? (
            current.model.services.map((s, i) => (
              <div
                key={i}
                className={'service-card ' + (dragOver === i ? 'drag-over' : '')}
                onDragOver={(e) => {
                  e.preventDefault()
                  if (dragOver !== i) setDragOver(i)
                }}
                onDragLeave={() => setDragOver(-1)}
                onDrop={(e) => dropDefault(e, i)}
              >
                <div className="service-head">
                  <b>{s.name || 'service'}</b>
                  <button type="button" className="link danger" onClick={() => removeService(i)}>
                    REMOVE
                  </button>
                </div>
                <div className="service-grid">
                  <label>
                    SERVICE NAME
                    <input
                      value={s.name}
                      onChange={(e) => updateService(i, (x) => ({ ...x, name: e.target.value }))}
                    />
                  </label>
                  <label>
                    IMAGE
                    <input
                      value={s.image || ''}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, image: e.target.value || null }))
                      }
                      placeholder="nginx:latest"
                    />
                  </label>
                  <label>
                    BUILD PATH (OPTIONAL)
                    <input
                      value={s.build || ''}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, build: e.target.value || null }))
                      }
                      placeholder="./app"
                    />
                  </label>
                  <label>
                    RESTART
                    <select
                      value={s.restart || ''}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, restart: e.target.value || null }))
                      }
                    >
                      <option value="unless-stopped">unless-stopped</option>
                      <option value="always">always</option>
                      <option value="on-failure">on-failure</option>
                      <option value="no">no</option>
                    </select>
                  </label>
                  <label className="full">
                    PORTS (ONE PER LINE)
                    <textarea
                      value={fromLines(s.ports)}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, ports: toLines(e.target.value) }))
                      }
                      placeholder="8080:80"
                    />
                  </label>
                  <label className="full">
                    VOLUMES (ONE PER LINE)
                    <textarea
                      value={fromLines(s.volumes)}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, volumes: toLines(e.target.value) }))
                      }
                      placeholder="/srv/data:/data"
                    />
                  </label>
                  <label className="full">
                    NETWORKS (ONE PER LINE)
                    <textarea
                      value={fromLines(s.networks)}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, networks: toLines(e.target.value) }))
                      }
                      placeholder="proxy"
                    />
                  </label>
                  <label className="full">
                    DEPENDS ON (ONE PER LINE)
                    <textarea
                      value={fromLines(s.depends_on)}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, depends_on: toLines(e.target.value) }))
                      }
                    />
                  </label>
                  <label className="full">
                    ENVIRONMENT (KEY=VALUE PER LINE)
                    <textarea
                      value={envToText(s.environment)}
                      onChange={(e) =>
                        updateService(i, (x) => ({
                          ...x,
                          environment: textToEnv(e.target.value, x.environment),
                        }))
                      }
                      placeholder="TZ=UTC"
                    />
                  </label>
                  <label className="full">
                    {'SECRET KEYS (COMMA SEPARATED → ${KEY} + .env)'}
                    <input
                      value={secretKeysOf(s.environment).join(', ')}
                      onChange={(e) =>
                        updateService(i, (x) => ({
                          ...x,
                          environment: applySecrets(x.environment, e.target.value),
                        }))
                      }
                      placeholder="API_KEY, DB_PASSWORD"
                    />
                  </label>
                  <label className="full">
                    COMMAND (OPTIONAL)
                    <input
                      value={s.command || ''}
                      onChange={(e) =>
                        updateService(i, (x) => ({ ...x, command: e.target.value || null }))
                      }
                    />
                  </label>
                </div>
              </div>
            ))
          ) : (
            <div className="muted">Add a service to begin, or drag a saved default here.</div>
          )}
        </section>

        <section className="section">
          <h2>📄 LIVE YAML</h2>
          {previewErr ? <div className="compose-err">{previewErr}</div> : null}
          <pre className="yaml-preview">
            {yaml || '# Add at least one valid service to see the YAML.'}
          </pre>
        </section>
      </div>
    </>
  )
}

/* ── Users ───────────────────────────────────────────────────────────────── */

function Users({ api }) {
  const copy = useCopy()
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
      <h2>{copy.discordUsers}</h2>
      <p className="fineprint">{copy.usersBlurb}</p>
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
                  {copy.noUsers}
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
  const copy = useCopy()
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
          label={copy.telegramService}
          value={
            <span className={'pill ' + (status?.configured ? 'done' : 'failed')}>
              {status?.configured ? 'ONLINE' : 'NOT SET'}
            </span>
          }
        />
        <Stat label={copy.notifyChatId} value={status?.chat_id || '—'} />
        <Stat label={copy.subscribers} value={status?.users ?? '—'} />
      </div>

      <section className="section highlight">
        <h2>{copy.telegramPanel}</h2>
        <p className="letter">{copy.telegramBlurb}</p>
      </section>

      <section className="section">
        <h2>{copy.telegramSubscribers}</h2>
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
    title: 'POSTGRESQL',
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
      ['YTDLP_PROXY', 'Optional proxy/VPN for geo-locked sites (e.g. US for Tubi).'],
    ],
    steps: [
      'Set DOWNLOADS_HOST_PATH to a host folder Sonarr/Radarr also mount (chown 10001:10001).',
      'Queue a URL from the Downloads tab; add a "Save as" name for clean matching.',
      'For Vimeo/login-gated videos, upload cookies.txt in the Downloads tab.',
      'Click 📥 IMPORT on a finished item to send it to Sonarr/Radarr.',
    ],
    usage: ['Downloads tab', 'Discord /download <url>', 'Telegram /download <url>'],
  },
  {
    key: 'compose',
    icon: '🧩',
    title: 'COMPOSE BUILDER',
    blurb:
      'Build compose projects visually and publish them for Dockhand (or any watcher) to pick up.',
    env: [
      ['COMPOSE_DIR', 'Container folder published files are written to (default /compose).'],
      ['COMPOSE_HOST_PATH', 'Host folder Dockhand watches; mounted at /compose.'],
    ],
    steps: [
      'Set COMPOSE_HOST_PATH to the folder Dockhand reads, then redeploy.',
      'On the server: mkdir -p <path> && chown 10001:10001 <path>.',
      'Open the COMPOSE tab → build/drag defaults → SAVE → PUBLISH.',
      'Published <name>.yml + <name>.env appear in the folder (secrets stay in .env).',
    ],
    usage: ['COMPOSE tab', 'Drag saved bind mounts / networks / env keys', 'Publish for Dockhand'],
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
    case 'compose':
      return st.compose ? { ok: true, text: 'READY' } : null
    case 'music':
      return { ok: null, text: 'CHECK MOUNT' }
    case 'deploy':
      return { ok: null, text: 'SEE STEPS' }
    default:
      return null
  }
}

function Setup({ api, theme, onTheme }) {
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

      <ThemeCard theme={theme} onTheme={onTheme} />
      <ChangePinCard api={api} />

      <div className="updated">STATUS SYNCED {updated}</div>
    </>
  )
}

function ThemeCard({ theme, onTheme }) {
  const copy = useCopy()
  return (
    <section className="section highlight">
      <h2>🎨 APPEARANCE</h2>
      <div className="form-grid">
        <label>
          DASHBOARD THEME
          <select value={theme} onChange={(e) => onTheme(e.target.value)}>
            <option value="419">{copy.theme419}</option>
            <option value="classic">{copy.themeClassic}</option>
          </select>
        </label>
      </div>
      <div className="muted small">Saved in this browser. Applies to the whole dashboard.</div>
    </section>
  )
}

function ChangePinCard({ api }) {
  const copy = useCopy()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirm, setConfirm] = useState('')
  const [msg, setMsg] = useState('')
  const [busy, setBusy] = useState(false)

  async function submit(e) {
    e.preventDefault()
    setMsg('')
    if (next !== confirm) {
      setMsg(copy.errPinMismatch)
      return
    }
    setBusy(true)
    try {
      await api.post('/api/auth/change-pin', { current_pin: current, new_pin: next })
      setMsg(copy.pinUpdated)
      setCurrent('')
      setNext('')
      setConfirm('')
    } catch {
      setMsg(copy.pinChangeFailed)
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
