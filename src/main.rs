//! Claude Code statusline: verbose `label → value` layout with rainbow value colors.

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

/// Best-effort sidecar: upsert this session's context-left into `~/.claude/ctx.db`
/// via an in-process SQLite (rusqlite, statically bundled — no external `sqlite3`
/// binary, portable to Win/Mac/Linux). One row per session; `count` bumps on every
/// render and `ts` tracks the last write, so a reader can tell a fresh-but-same-%
/// sample from a stale one. Every error is swallowed: the bar must render even if
/// the DB is locked or the disk is full.
fn persist(session_id: &str, remaining_pct: f64) {
    if session_id.is_empty() || session_id == "?" {
        return;
    }
    let _ = persist_inner(session_id, remaining_pct);
}

fn persist_inner(session_id: &str, remaining_pct: f64) -> rusqlite::Result<()> {
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
         CREATE TABLE IF NOT EXISTS ctx(session_id TEXT PRIMARY KEY, count INTEGER NOT NULL, ts INTEGER NOT NULL, remaining_pct REAL);",
    )?;
    // count++ on every render; one row per session.
    conn.execute(
        "INSERT INTO ctx(session_id, count, ts, remaining_pct) VALUES(?1, 1, ?2, ?3)\
         ON CONFLICT(session_id) DO UPDATE SET count = count + 1, ts = ?2, remaining_pct = ?3",
        rusqlite::params![session_id, ts, remaining_pct],
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
    let reset_7d     = data["rate_limits"]["seven_day"]["resets_at"].as_u64().map(format_reset_time).unwrap_or_else(|| String::from("?"));

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
    let rate_7d_display = if rate_7d <= 20.0 {
        format!("{rate_7d:.0}% (reset at {reset_7d})")
    } else {
        format!("{rate_7d:.0}%")
    };

    let fields = [
        format!("{}{slash}{}", colorize("red", model_name), colorize("orange", effort_level)),
        format!("{} {arrow} {}", label("ctx_left"),   colorize("yellow", &format!("{context_pct:.1}%"))),
        format!("{} {arrow} {}", label("cwd"),        colorize("green",  &cwd_display)),
        format!("{} {arrow} {}", label("5h_left"),  colorize("blue",   &rate_5h_display)),
        format!("{} {arrow} {}", label("7d_left"),  colorize("blue",   &rate_7d_display)),
        format!("{} {arrow} {}", label("session_id"), colorize("violet", session_id)),
    ];

    let joiner = format!(" {sep} ");
    print!("{}", fields.join(&joiner));

    // Persist after rendering so the visible bar is never delayed by the write.
    persist(session_id, context_pct);
}
