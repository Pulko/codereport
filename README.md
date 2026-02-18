# codereport

Repo-scoped CLI for code reports: track TODOs, refactors, 
bugs, and critical items with tags, expiration, and CI 
checks.

**Track code follow-ups where they live—with tags, ownership, expiration, and CI.**

codereport is a repo-scoped CLI that turns scattered TODOs, refactors, bugs, and critical items into a single, auditable list. Add reports by file and line range, attach severity and optional expiration, and gate merges with a CI check. A generated HTML dashboard gives your team at-a-glance heatmaps and stats so nothing slips.

---

## Why codereport?

- **Scattered follow-ups** — TODOs and “fix later” comments live in code or in tickets, with no single place to see what’s open, who owns it, or when it’s due.
- **No visibility** — There’s no quick way to see which files or areas carry the most tech debt, bugs, or critical items.
- **No gates** — Critical or long-overdue items can ship because nothing fails the build when they’re still open.

codereport keeps all of this in one place: a repo-owned list of reports, keyed by file and line range, with tags, messages, ownership, and optional expiration. You run it from the repo, generate a dashboard when you need it, and use `codereport check` in CI to block on blocking or expired items.

---

## Key benefits

- **One source of truth** — All code follow-ups (TODOs, refactors, bugs, critical) in a single YAML store under `.codereports/`, versioned with the repo.
- **Precise location** — Every report is tied to a path and line range (`path:start-end`), so you know exactly where to look.
- **Severity and expiration** — Tag types map to severity (low → blocking). You can set expiration per tag (e.g. critical: 14 days, buggy: 90 days). The dashboard and CI both respect “expired” and “expiring soon.”
- **Ownership** — On add, ownership is resolved from CODEOWNERS or git blame (with a local cache), so each report can be attributed without extra work.
- **CI gate** — `codereport check` exits non-zero if any open report is blocking severity or past its expiration date. Use it in CI to prevent merging with open critical or stale items.
- **Dashboard, not a long table** — `codereport html` generates a minimal dashboard: KPIs (total, open, resolved, critical, expired, expiring soon), tag distribution, and a file × tag heatmap. No need to scroll a giant list to see where the work is.
- **Fits your workflow** — Add and resolve from the CLI; list and filter by tag or status. Config is in the repo (tags, severity, expiration), so policy is explicit and reviewable.

---

## Features

| Feature | Description |
|--------|-------------|
| **Tagged reports** | todo, refactor, buggy, critical — configurable severity and optional expiration per tag |
| **Line-range scoped** | Reports are tied to `path:start-end` (e.g. `src/foo.rs:42-88`) |
| **Configurable policy** | `.codereports/config.yaml` defines severity (low / medium / high / blocking) and expiration days per tag; default: critical 14d, buggy 90d, refactor 180d, todo no expiry |
| **Ownership** | CODEOWNERS first, then git blame for the line range; result (git + codeowner) stored on each report; blame cached locally |
| **CI check** | `codereport check` fails if any open report is blocking or expired — use in PR/merge checks |
| **HTML dashboard** | Stats, tag bars, file × tag heatmap (top 30 files); dark, minimal UI; generated under `.codereports/html/` (gitignored) |
| **Git-friendly** | Only `reports.yaml` and `config.yaml` are meant to be committed; generated HTML and blame cache are ignored |

---

## Use cases

- **Code review follow-ups** — Add a report instead of a vague “address this later” comment; assign severity and optional expiry; track in one place.
- **Tech debt and refactors** — Use the refactor tag with an expiration so refactors don’t sit forever; see which files have the most in the heatmap.
- **Known bugs** — Tag buggy or critical with expiration; CI can block until they’re resolved or extended.
- **Critical / blocking items** — Mark as critical with a short expiry; `codereport check` in CI ensures they can’t be merged while open.
- **Team visibility** — Generate the HTML dashboard for standups or planning; heatmap and stats show hotspots and trends without opening every file.

---

## Quick start

From the repo root:

```bash
codereport init
```

This creates `.codereports/` with `config.yaml` and updates the repo root `.gitignore` so `.codereports/html/` and `.codereports/.blame-cache` are ignored.

---

## Commands

| Command | Description |
|--------|-------------|
| `codereport add <path>:<start>-<end> --tag <tag> --message <text>` | Add a report (tag: todo, refactor, buggy, critical) |
| `codereport list [--tag <tag>] [--status open\|resolved]` | List reports with optional filters |
| `codereport delete <id>` | Delete by ID (e.g. CR-000001) |
| `codereport resolve <id>` | Mark as resolved |
| `codereport check` | CI: exit 1 if any open report is blocking or expired |
| `codereport html [--no-open]` | Generate `.codereports/html/index.html` and open in browser (use `--no-open` to skip open) |

---

## Usage in CI

`codereport check` exits with code 1 if any **open** report is either:

- **Blocking** — its tag has severity `blocking` in `config.yaml` (e.g. default `critical`), or  
- **Expired** — it has an `expires_at` date and that date is before today (CI uses the runner’s local date).

Resolved reports are ignored. When the check fails, it prints each violating report to stderr as: `ID  path  tag  message`. Fix by resolving or deleting those reports, or by updating expiration where appropriate.

### Installing in CI

**GitHub Actions:** The easiest way is the [codereport action](action.yml) — it installs codereport and runs `check` (or another command) in one step. See the example below.

**Other CI / manual install:** Install the binary before running `check`:

- **From crates.io:** `cargo install codereport` (ensure `cargo` is available in the job).
- **From source:** clone the repo and run `cargo build --release`; use the binary from `target/release/codereport`.

Then run from the **repository root** (where `.codereports/` and `reports.yaml` live):

```bash
codereport check
```

### Example: GitHub Actions

Using the **codereport action** (recommended):

```yaml
name: codereport
on:
  pull_request:
    branches: [main]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Pulko/codereport/action@v1
        # Optional: version: '0.1.2'  # pin to a release; default is 'latest'
```

Add this job to your workflow so PRs cannot merge while blocking or expired reports are open. To run other commands (`init`, `list`, `html`), set the `command` input and optionally `arguments` (e.g. `arguments: '--tag critical --status open'` for `list`).

**Manual install** (no action):

```yaml
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo install codereport
      - run: codereport check
```

### Example: GitLab CI

```yaml
codereport:
  stage: test
  image: rust:latest
  script:
    - cargo install codereport
    - codereport check
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

Run in the same branch as the MR; the job fails if the check fails.

### When to run

- **On every PR / MR** — Ensures no new merge introduces open blocking or expired items; existing reports are already in the repo.
- **On main / default branch** — Optional; useful to catch reports that were opened after merge or to enforce policy on a schedule.

If your CI doesn’t have Rust, build `codereport` in a separate “build” job and cache the binary, or use a pre-built release binary and install that in the job instead of `cargo install`.

---

## What to track in Git

**Commit:**

- `.codereports/reports.yaml` — report data
- `.codereports/config.yaml` — tag and policy config

**Do not commit (ignored via repo root `.gitignore`):**

- `.codereports/html/` — generated dashboard
- `.codereports/.blame-cache` — local blame cache (per-machine, not shared)
