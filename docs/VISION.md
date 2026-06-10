# Vision — an all-in-one self-hosted control platform

NigerianBot is the first surface of a broader goal: a single **self-hosted control
plane** for a home server, with two faces over **one Rust core + one database**:

- **Bot** — the Discord-facing surface (commands, voice, notifications).
- **Dashboard (UI)** — settings, administration, and control.

> The bot is for Discord; the UI is for settings and control. Same backend,
> same data, two front doors.

## North star

When a person **signs up or links their Discord account**, the platform can
**provision and manage their access across connected self-hosted services** —
with roles and privileges that evolve over time. Concrete targets on the roadmap:

- **Authentik** — central identity provider (SSO / OIDC). The source of truth for
  who someone is.
- **Seer** (Jellyseerr → migrating to Overseerr) — media requests.
- Other Docker-hosted apps — create / link / disable accounts, set permissions.

So a flow like: *user links Discord → an Authentik identity is created/linked →
accounts are provisioned on the right services at the right privilege level →
all manageable from the dashboard, and surfaced/triggered from Discord.*

## Foundation principles (build for this, even when shipping small features)

1. **One core, many surfaces.** Keep identity, config, database access, and
   service integrations in **shared crates** (`common`, a services layer) — never
   bot-only. The bot and the API/UI are thin surfaces over that core.
2. **Identity model first.** A `users` table linking *Discord ID ↔ platform
   account ↔ external service accounts*, with roles. RBAC flows through the API's
   JWT (`role` claim already exists).
3. **Uniform service connectors.** New integrations follow the existing `Arr`
   client pattern (base URL + key/credentials + typed client + uniform error
   handling), so Authentik, Seer, and future apps plug in consistently and are
   testable in isolation.
4. **SSO-ready auth.** Plan for Authentik-issued (OIDC) identities to flow into
   the API's authentication alongside the current API-key path.
5. **Provisioning as privileged actions.** Account create/link/disable operations
   live behind admin roles and are audited (the `audit_log` already exists).
6. **Rust for scale & simple ops.** Performance, safety, and a single deployable
   per surface; the platform should stay lightweight on a home server.

## Near-term steps that lay the foundation

- **RBAC** (admin / viewer) enforced in both the API and the bot.
- A **`users` / account model** with Discord account linking.
- A **`ServiceConnector` abstraction** generalising the `Arr` client; Authentik
  and Seer as the first new connectors.
- **Account provisioning actions** (create / link / disable) behind admin
  privileges, driven from the dashboard and/or `/admin` commands.

## How to use this doc

When prioritising or designing a feature, prefer the option that **generalises
toward the platform** over a one-off bot feature. If a choice makes future
identity/provisioning work easier (shared crates, a connector abstraction, a
real user model), that's the one to take.
