---
name: ctx-left
description: Report a Claude Code session's remaining context-window %, plus the statusline render `count` and the age of the last sample, so a fresh-but-same-% reading is distinguishable from a stale one. Defaults to the current session ($CLAUDE_CODE_SESSION_ID). Reads ~/.claude/ctx.db, which the Rust statusline upserts on every render.
---

# ctx-left

Reports context-window usage for a Claude Code session from `~/.claude/ctx.db`
(written by the statusline on every render: one upserted row per session holding
`session_id, count, ts, remaining_pct`).

## Run

The `ctx-left` binary ships with the statusline (same `cargo build`, same bundled
SQLite) and is expected on `PATH` — `install.sh` / `install.ps1` put it there via
`cargo install`. No Python or other runtime needed.

- **Current session:** `ctx-left`
- **A specific session:** `ctx-left <session-uuid>`
- **All sessions:** `ctx-left --all`

If `ctx-left` is not on `PATH`, call it by absolute path
(`~/.cargo/bin/ctx-left`, or `<repo>/target/release/ctx-left`).

Report the command's output to the user.

## Reading the output

Each line is `ctx_left <pct>% · <count> render · ultimo campione <N>s fa`:

- **pct** — remaining context %, the same integer the statusline shows.
- **count** — how many times the statusline has rendered for that session.
- **age** — seconds since the last write.

If two reads show the **same %** but a higher `count` / fresher age, the session is
live and consuming tokens — just under the statusline's ~1-point resolution
(≈1% of the window, which is ~10k tokens on a 1M-token model).
