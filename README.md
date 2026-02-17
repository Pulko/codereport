# codereport

Repo-scoped CLI for code reports: track TODOs, refactors, bugs, and critical items with tags, expiration, and CI checks.

## Setup

From the repo root:

```bash
codereport init
```

This creates `.codereports/` with `config.yaml`, and adds or creates the **repo root** `.gitignore` so that `.codereports/html/` and `.codereports/.blame-cache` are ignored (generated dashboard and local blame cache).

## Commands

- `codereport add <path>:<start>-<end> --tag <tag> --message <text>` — add a report (tag: todo, refactor, buggy, critical)
- `codereport list [--tag <tag>] [--status open|resolved]` — list reports
- `codereport delete <id>` — delete by ID (e.g. CR-000001)
- `codereport resolve <id>` — mark as resolved
- `codereport check` — CI: exits 1 if any open report is blocking or expired
- `codereport html` — generate `.codereports/html/index.html` and open in browser (use `--no-open` to skip open)

## Git

Track these in version control:

- `.codereports/reports.yaml` — report data
- `.codereports/config.yaml` — tag and policy config

Do **not** track (ignored via repo root `.gitignore`):

- `.codereports/html/` — generated dashboard
- `.codereports/.blame-cache` — local blame cache (per-user, not tracked)
