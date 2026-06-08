//! bridge-ls — list Claude Code sessions on the bridge with their 3-letter bridge
//! name, context-left %, and working dir.
//!
//! Reads `~/.claude/ctx.db` (written by the statusline, which mirrors the bridge's
//! `sessionNames`/`sessionRooms` into the `bridge_name`/`bridge_room` columns).
//! Ships with the statusline (same crate, same bundled rusqlite): one `cargo build`
//! produces every binary, no Python or other runtime. Read-only.
//!
//! Usage:  bridge-ls          list sessions with a bridge name (default)
//!         bridge-ls --all    list every session (blank name shown as "—")

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

fn home_dir() -> Option<String> {
    // $HOME on unix, %USERPROFILE% on Windows.
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
}

fn die(msg: &str, code: i32) -> ! {
    eprintln!("{msg}");
    std::process::exit(code);
}

/// Fold `$HOME` → `~` for compact display.
fn fold_home(path: &str, home: &str) -> String {
    if path == home {
        "~".to_string()
    } else if let Some(rest) = path.strip_prefix(home).filter(|rest| rest.starts_with('/')) {
        format!("~{rest}")
    } else {
        path.to_string()
    }
}

/// `42.5` → "42.5%", `88.0` → "88%", missing → "?".
fn fmt_pct(pct: Option<f64>) -> String {
    match pct {
        Some(p) if p.fract() == 0.0 => format!("{}%", p as i64),
        Some(p) => format!("{p}%"),
        None => "?".to_string(),
    }
}

fn main() {
    let all = std::env::args().skip(1).any(|a| a == "--all");

    let Some(home) = home_dir() else { die("HOME/USERPROFILE non impostata", 2) };
    let db = format!("{home}/.claude/ctx.db");
    if !Path::new(&db).exists() {
        die(
            &format!("DB assente: {db} (la statusline non ha ancora scritto nulla)"),
            1,
        );
    }
    let conn = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap_or_else(|e| die(&format!("apertura DB fallita: {e}"), 1));

    // Most recently rendered first, so live sessions surface at the top.
    let sql = if all {
        "SELECT bridge_name, remaining_pct, cwd FROM ctx ORDER BY ts DESC"
    } else {
        "SELECT bridge_name, remaining_pct, cwd FROM ctx WHERE bridge_name IS NOT NULL ORDER BY ts DESC"
    };

    let mut stmt = conn
        .prepare(sql)
        .unwrap_or_else(|e| die(&format!("query fallita: {e}"), 1));
    let rows: Vec<(String, String, String)> = stmt
        .query_map([], |r| {
            let name: Option<String> = r.get(0)?;
            let pct: Option<f64> = r.get(1)?;
            let cwd: Option<String> = r.get(2)?;
            Ok((
                name.unwrap_or_else(|| "—".to_string()),
                fmt_pct(pct),
                cwd.map(|c| fold_home(&c, &home))
                    .unwrap_or_else(|| "?".to_string()),
            ))
        })
        .and_then(Iterator::collect)
        .unwrap_or_else(|e| die(&format!("lettura righe fallita: {e}"), 1));

    if rows.is_empty() {
        if all {
            die("nessuna sessione registrata", 1);
        }
        die(
            "nessuna sessione bridge registrata\n\
             (i server bridge scrivono il nome al prossimo avvio della sessione)",
            1,
        );
    }

    // Align the name + ctx columns; cwd is last so it needs no padding.
    let w_name = rows
        .iter()
        .map(|(n, _, _)| n.chars().count())
        .max()
        .unwrap_or(0)
        .max("bridge".len());
    let w_pct = rows
        .iter()
        .map(|(_, p, _)| p.chars().count())
        .max()
        .unwrap_or(0)
        .max("ctx".len());

    println!("{:<w_name$}  {:>w_pct$}  {}", "bridge", "ctx", "cwd");
    for (name, pct, cwd) in &rows {
        println!("{name:<w_name$}  {pct:>w_pct$}  {cwd}");
    }
}
