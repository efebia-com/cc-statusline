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

## Verdict: soglie di degrado (modello 1M)

Su un context da 1M la qualità **non** regge piatta fino a zero — degrada molto prima
(a 80% rimasta hai già bruciato ~200k token, cioè un intero vecchio context da 200k).
Soglie empiriche (osservate dall'utente, modello 1M; lette su `ctx_left` = % **rimasta**):

| `ctx_left` | stato | azione |
|-----------|-------|--------|
| **90–100%** | 🟢 ok | piena capacità, lavora tranquillo |
| **85–89%** | 🟡 attenzione | ok, ma **non iniziare task enormi** |
| **80–84%** | 🟠 niente task nuovi | **non iniziare nuovi task**; porta a termine quelli in corso |
| **75–79%** | 🔴 valuta interruzione | valuta se interrompere i task in corso oppure annotarli nel **`/next`**; se sono **piccoli e critical** è meglio finirli subito |
| **70–74%** | ⛔ forza handoff | cerca di **forzare l'handoff**: lancia **`/next`** ORA e prepara il passaggio a una chat nuova |
| **≤ 69%** | ☠️ allucinazioni | il lavoro fatto qui è a rischio: nell'handoff **dichiara esplicitamente** che probabilmente contiene allucinazioni o ha addirittura deviato dal compito → serve **verifica e controllo umano** |

Dopo aver riportato la %, **classifica sempre** nella banda e applica/raccomanda
l'azione corrispondente. **NON** dare un "c'è margine, continua pure" senza riserve
sotto il 90%: la curva è ripida, non lineare. Da ⛔ in giù la raccomandazione è
decisa, non un'opzione neutra ("dimmi tu"); la chat nuova può poi ricostruire il
contesto anche con **`/handoff:handoff <session-id>`**.

Esempio — `ctx_left 47% · …` cade in ☠️, quindi **non** "c'è margine":

> **Context: 47% rimasto — ☠️ zona allucinazioni (≤ 69%).** Non continuare lavoro
> importante qui: lancia **`/next`** e nell'handoff segnala che il lavoro svolto in
> questa sessione va **verificato da un umano** — probabile presenza di allucinazioni
> o deviazioni dal compito.
