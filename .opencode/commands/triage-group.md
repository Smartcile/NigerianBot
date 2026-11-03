---
description: Triage several related issues into one grouped fix plan
agent: triager
---
Triage GitHub issues $ARGUMENTS as one related group.

If `$ARGUMENTS` is empty, ask for the issue numbers and stop.

Follow the triage process in your agent prompt:
- read `CLAUDE.md` and `docs/ROADMAP.md`
- read every issue and its comments
- investigate the code read-only
- decide whether the issues genuinely share a root cause, files, or user flow
- rename every issue to a clean `[AREA] Short imperative description` title with the same area prefix
- write one combined plan to `.agent/triage-group-<first-issue>.md`
- post it on the lead issue and link the other issues
- add `agent:planned` to every issue
- report whether you recommend one grouped branch/PR or separate branches

Do not implement or open a PR.
