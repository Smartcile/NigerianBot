# Deployment & networking

NigerianBot is designed to drop into **your** infrastructure rather than impose
its own. It serves plain HTTP and expects you to bring your own reverse proxy /
TLS if you want a public, HTTPS URL — exactly like Sonarr, Radarr, Jellyfin, etc.

## What's exposed

| Service | Direction | Port | Needs exposing? |
|---------|-----------|------|-----------------|
| **bot** | **outbound only** (connects out to Discord) | — | ❌ never |
| **api + dashboard** | inbound HTTP | `API_PORT` (default `8000`) | only if you want to reach the dashboard/API |
| **postgres** | internal | 5432 | ❌ keep internal |

The Discord bot **never needs an inbound port, forwarded port, or reverse
proxy** — it dials out to Discord. Only the API/dashboard is a web surface, and
only if you choose to reach it.

## Just using it locally?

Open `http://<server-lan-ip>:8000/` and sign in with your `API_KEY`. Plain HTTP
on your LAN is perfectly fine for personal use — no proxy required.

## Putting it behind HTTPS (bring your own proxy)

Point your existing reverse proxy at `http://<server-ip>:8000`. The dashboard and
API are the same origin, so there's nothing special to configure.

### Nginx Proxy Manager
1. **Hosts → Proxy Hosts → Add Proxy Host**
2. Domain: `bot.example.com` · Scheme: `http` · Forward host: `<server-ip>` ·
   Forward port: `8000`
3. **SSL** tab → request a Let's Encrypt cert → Force SSL. Done.

### Caddy
```caddy
bot.example.com {
    reverse_proxy <server-ip>:8000
}
```

### Traefik (compose labels on the `api` service)
```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.nigerianbot.rule=Host(`bot.example.com`)"
  - "traefik.http.services.nigerianbot.loadbalancer.server.port=8000"
```

### Cloudflare Tunnel
Create a tunnel with a public hostname routing to `http://<server-ip>:8000`.
(Note: this is for the **dashboard/API** only — Discord *voice* is outbound UDP
and must not be routed through a tunnel.)

## Why no proxy is bundled

A bundled reverse proxy would force one networking opinion onto every operator
and stack a second proxy behind ones people already run (NPM, Traefik, Caddy,
Cloudflare). Keeping the app to plain HTTP makes it portable: front it however
*you* like, or not at all.

## Security notes

- The dashboard/API require a JWT obtained from `API_KEY`; keep that secret.
- If you expose the API publicly, put HTTPS in front of it (above) so the API key
  and tokens aren't sent in clear text.
- Discord slash commands can be restricted per role/channel in **Server Settings
  → Integrations** if you want to limit who can use `add`/admin-style commands.
