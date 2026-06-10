import { useState, useEffect, useCallback } from 'react'

const TOKEN_KEY = 'nb_token'

export default function App() {
  const [token, setToken] = useState(() => localStorage.getItem(TOKEN_KEY))

  const login = useCallback((t) => {
    localStorage.setItem(TOKEN_KEY, t)
    setToken(t)
  }, [])

  const logout = useCallback(() => {
    localStorage.removeItem(TOKEN_KEY)
    setToken(null)
  }, [])

  return token ? <Dashboard token={token} onLogout={logout} /> : <Login onLogin={login} />
}

function Login({ onLogin }) {
  const [key, setKey] = useState('')
  const [err, setErr] = useState('')

  async function submit(e) {
    e.preventDefault()
    setErr('')
    try {
      const r = await fetch('/api/auth/login', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ api_key: key }),
      })
      if (!r.ok) {
        setErr('Invalid API key.')
        return
      }
      const data = await r.json()
      onLogin(data.token)
    } catch {
      setErr('Could not reach the server.')
    }
  }

  return (
    <div className="login">
      <form className="card" onSubmit={submit}>
        <h1>🎵 NigerianBot</h1>
        <p>Sign in with your API key.</p>
        <input
          type="password"
          placeholder="API key"
          value={key}
          onChange={(e) => setKey(e.target.value)}
        />
        <button type="submit">Sign in</button>
        <div className="err">{err}</div>
      </form>
    </div>
  )
}

function Dashboard({ token, onLogout }) {
  const [status, setStatus] = useState(null)
  const [stats, setStats] = useState(null)
  const [logs, setLogs] = useState([])
  const [updated, setUpdated] = useState('')

  const api = useCallback(
    async (path) => {
      const r = await fetch(path, { headers: { Authorization: 'Bearer ' + token } })
      if (r.status === 401) {
        onLogout()
        throw new Error('unauthorized')
      }
      return r.json()
    },
    [token, onLogout],
  )

  const refresh = useCallback(async () => {
    try {
      const [st, stt, lg] = await Promise.all([
        api('/api/bot/status'),
        api('/api/stats'),
        api('/api/bot/logs?limit=25'),
      ])
      setStatus(st)
      setStats(stt)
      setLogs(lg.logs || [])
      setUpdated(new Date().toLocaleTimeString())
    } catch {
      /* logout handled in api() */
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
      <header>
        <h1>🎵 NigerianBot Dashboard</h1>
        <button onClick={onLogout}>Sign out</button>
      </header>
      <main>
        <div className="grid">
          <Stat label="Total commands" value={stats?.total ?? '—'} />
          <Stat label="Last 24 hours" value={stats?.last_24h ?? '—'} />
          <Stat
            label="Database"
            value={<span className={'pill ' + (dbOk ? 'good' : 'bad')}>{dbOk ? 'Connected' : 'Down'}</span>}
          />
        </div>

        <section className="section">
          <h2>TOP COMMANDS</h2>
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
            <div className="muted">No commands yet.</div>
          )}
        </section>

        <section className="section">
          <h2>RECENT ACTIVITY</h2>
          <table>
            <thead>
              <tr>
                <th>When</th>
                <th>User</th>
                <th>Command</th>
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
                    No activity yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </section>

        <div className="foot">Auto-refreshes every 10s · updated {updated}</div>
      </main>
    </>
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
