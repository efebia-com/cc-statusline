//! Claude Code statusline: verbose `label → value` layout with rainbow value colors.

use chrono::Local;
use rusqlite::Connection;
use serde_json::Value;
use std::io::{self, Read};
use std::time::{SystemTime, UNIX_EPOCH};

fn colorize(name: &str, text: &str) -> String {
    let code: u8 = match name {
        "gray"   => 246,
        // Rainbow palette (ROYGBV), bright variants for legibility on dark bg
        "red"    => 196,
        "orange" => 208,
        "yellow" => 220,  // gold (#ffd700) — softer than pure 226 (#ffff00)
        "green"  => 71,   // sage (#5faf5f) — softer than pure 46 (#00ff00)
        "blue"   => 33,
        "violet" => 140,  // lavender (#af87d7) — softer than pure 129 (#af00ff)
        _ => panic!("unknown color: {name}"),
    };
    format!("\x1b[38;5;{code}m{text}\x1b[0m")
}

fn format_reset_time(resets_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let seconds_left = resets_at.saturating_sub(now);

    let days = seconds_left / 86_400;
    let hours = (seconds_left % 86_400) / 3_600;
    let minutes = (seconds_left % 3_600) / 60;

    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else if seconds_left > 0 {
        String::from("<1m")
    } else {
        String::from("now")
    }
}

fn format_reset_compact(resets_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let seconds_left = resets_at.saturating_sub(now);

    let days = seconds_left / 86_400;
    let hours = (seconds_left % 86_400) / 3_600;
    let minutes = (seconds_left % 3_600) / 60;

    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else if seconds_left > 0 {
        String::from("<1m")
    } else {
        String::from("now")
    }
}

/// Best-effort: pull this session's bridge identity from the bridge plugin's
/// `~/.claude/channels/bridge/config.json`. Returns `(3-letter name, current room)`,
/// each `None` when the session isn't using the bridge (no entry) or the file is
/// missing/unreadable. The bridge writes `sessionNames`/`sessionRooms` keyed by the
/// same Claude session UUID the statusline gets, so no recomputation is needed.
fn read_bridge(home: &str, session_id: &str) -> (Option<String>, Option<String>) {
    let text = match std::fs::read_to_string(format!("{home}/.claude/channels/bridge/config.json")) {
        Ok(text) => text,
        Err(_) => return (None, None),
    };
    let cfg: Value = match serde_json::from_str(&text) {
        Ok(cfg) => cfg,
        Err(_) => return (None, None),
    };
    let lookup = |key: &str| {
        cfg.get(key)
            .and_then(|map| map.get(session_id))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    (lookup("sessionNames"), lookup("sessionRooms"))
}

/// Best-effort sidecar: upsert this session's context-left into `~/.claude/ctx.db`
/// via an in-process SQLite (rusqlite, statically bundled — no external `sqlite3`
/// binary, portable to Win/Mac/Linux). One row per session; `count` bumps on every
/// render and `ts` tracks the last write, so a reader can tell a fresh-but-same-%
/// sample from a stale one. `model`/`effort`/`cwd` snapshot the session's current
/// model, effort level, and working dir. Every error is swallowed: the bar must
/// render even if the DB is locked or the disk is full.
fn persist(session_id: &str, remaining_pct: f64, model: &str, effort: &str, cwd: &str) {
    if session_id.is_empty() || session_id == "?" {
        return;
    }
    let _ = persist_inner(session_id, remaining_pct, model, effort, cwd);
}

fn persist_inner(
    session_id: &str,
    remaining_pct: f64,
    model: &str,
    effort: &str,
    cwd: &str,
) -> rusqlite::Result<()> {
    // $HOME on unix, %USERPROFILE% on Windows.
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    if home.is_empty() {
        return Ok(());
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let conn = Connection::open(format!("{home}/.claude/ctx.db"))?;
    // WAL + busy_timeout let concurrent sessions write safely, no external locks.
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA busy_timeout=3000; PRAGMA synchronous=NORMAL;\
         CREATE TABLE IF NOT EXISTS ctx(session_id TEXT PRIMARY KEY, count INTEGER NOT NULL, ts INTEGER NOT NULL, remaining_pct REAL, model TEXT, effort TEXT, cwd TEXT, bridge_name TEXT, bridge_room TEXT);",
    )?;
    // Migrate DBs created before these columns existed. On an already-migrated DB each
    // ALTER fails with "duplicate column" — expected, so each one is best-effort.
    for col in ["model", "effort", "cwd", "bridge_name", "bridge_room"] {
        let _ = conn.execute(&format!("ALTER TABLE ctx ADD COLUMN {col} TEXT"), []);
    }
    // Pull this session's bridge identity (3-letter name + current room), if any.
    let (bridge_name, bridge_room) = read_bridge(&home, session_id);
    // count++ on every render; one row per session.
    conn.execute(
        "INSERT INTO ctx(session_id, count, ts, remaining_pct, model, effort, cwd, bridge_name, bridge_room) VALUES(?1, 1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)\
         ON CONFLICT(session_id) DO UPDATE SET count = count + 1, ts = ?2, remaining_pct = ?3, model = ?4, effort = ?5, cwd = ?6, bridge_name = ?7, bridge_room = ?8",
        rusqlite::params![session_id, ts, remaining_pct, model, effort, cwd, bridge_name, bridge_room],
    )?;
    // prune sessions idle > 7 days (604800s).
    conn.execute(
        "DELETE FROM ctx WHERE ts < ?1",
        rusqlite::params![ts - 604_800],
    )?;
    Ok(())
}

fn main() {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).expect("read stdin");
    let data: Value = serde_json::from_str(&buf).expect("parse JSON");

    let model_name   = data["model"]["display_name"].as_str().unwrap_or("?");
    let cwd          = data["cwd"].as_str().unwrap_or("?");
    let session_id   = data["session_id"].as_str().unwrap_or("?");
    let context_pct  = data["context_window"]["remaining_percentage"].as_f64().unwrap_or(100.0);
    let effort_level = data["effort"]["level"].as_str().unwrap_or("?");
    let rate_5h      = 100.0 - data["rate_limits"]["five_hour"]["used_percentage"].as_f64().unwrap_or(0.0);
    let rate_7d      = 100.0 - data["rate_limits"]["seven_day"]["used_percentage"].as_f64().unwrap_or(0.0);
    let reset_5h     = data["rate_limits"]["five_hour"]["resets_at"].as_u64().map(format_reset_time).unwrap_or_else(|| String::from("?"));
    let reset_7d     = data["rate_limits"]["seven_day"]["resets_at"].as_u64().map(format_reset_compact).unwrap_or_else(|| String::from("?"));

    // Fold $HOME → ~
    let home = std::env::var("HOME").unwrap_or_default();
    let cwd_display = match cwd.strip_prefix(&home) {
        Some("") => String::from("~"),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => cwd.to_string(),
    };

    let arrow = colorize("gray", "→");
    let sep   = colorize("gray", "│");
    let label = |s: &str| colorize("gray", s);

    let slash = colorize("gray", "/");
    let rate_5h_display = if rate_5h <= 30.0 {
        format!("{rate_5h:.0}% (reset at {reset_5h})")
    } else {
        format!("{rate_5h:.0}%")
    };
    let rate_7d_display = format!("{rate_7d:.0}% ({reset_7d})");

    // Render-time stamp (local): each render = a message/inject, so this shows
    // at a glance when the line was last updated. %a is the English weekday abbrev.
    let now_display = Local::now().format("%H:%M:%a").to_string();

    let fields = [
        colorize("gray", &now_display),
        format!("{}{slash}{}", colorize("red", model_name), colorize("orange", effort_level)),
        format!("{} {arrow} {}", label("ctx_left"),   colorize("yellow", &format!("{context_pct:.1}%"))),
        format!("{} {arrow} {}", label("cwd"),        colorize("green",  &cwd_display)),
        format!("{} {arrow} {}", label("5h"),  colorize("blue",   &rate_5h_display)),
        format!("{} {arrow} {}", label("7d"),  colorize("blue",   &rate_7d_display)),
        format!("{} {arrow} {}", label("id"), colorize("violet", session_id)),
    ];

    let joiner = format!(" {sep} ");
    print!("{}", fields.join(&joiner));

    // Persist after rendering so the visible bar is never delayed by the write.
    // Store the raw (un-folded) cwd — the canonical absolute path, not the ~ display.
    persist(session_id, context_pct, model_name, effort_level, cwd);
}
