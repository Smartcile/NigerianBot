---
description: Triage a GitHub issue and post a fix plan
agent: triager
---
Triage GitHub issue $1.

If `$1` is empty, ask for the issue number and stop.

Follow the triage process in your agent prompt:
- read `CLAUDE.md` and `docs/ROADMAP.md`
- read the issue and all comments
- investigate the code read-only
- rename the issue title to `[AREA] Short imperative description`
- write `.agent/triage-$1.md`
- post it with `gh issue comment $1 --body-file .agent/triage-$1.md`
- add the `agent:planned` label
- report the issue number, new title, likely files, and any schema impact
