//! ctx-left — read `~/.claude/ctx.db` (written by the statusline) and report context-left.
//!
//! Ships with the statusline (same crate, same bundled rusqlite): one `cargo build`
//! produces both binaries, with no Python or other runtime dependency. Read-only.

use rusqlite::{Connection, OpenFlags, Row};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn home_dir() -> Option<String> {
    // $HOME on unix, %USERPROFILE% on Windows.
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn fmt(sid: &str, count: i64, ts: i64, pct: f64) -> String {
    let age = (now() - ts).max(0);
    let pct_s = if pct.fract() == 0.0 {
        format!("{}", pct as i64)
    } else {
        format!("{pct}")
    };
    format!("{sid}\n   ctx_left {pct_s}%  ·  {count} render  ·  ultimo campione {age}s fa")
}

fn row_to_line(r: &Row) -> rusqlite::Result<String> {
    Ok(fmt(&r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
}

fn die(msg: &str, code: i32) -> ! {
    eprintln!("{msg}");
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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

    const SQL_ALL: &str = "SELECT session_id,count,ts,remaining_pct FROM ctx ORDER BY ts DESC";
    const SQL_ONE: &str = "SELECT session_id,count,ts,remaining_pct FROM ctx WHERE session_id=?1";

    if args.first().map(String::as_str) == Some("--all") {
        let mut stmt = conn.prepare(SQL_ALL).unwrap();
        let lines: Vec<String> = stmt.query_map([], row_to_line).unwrap().flatten().collect();
        if lines.is_empty() {
            die("nessuna sessione registrata", 1);
        }
        println!("{}", lines.join("\n"));
        return;
    }

    let sid = args
        .into_iter()
        .next()
        .or_else(|| std::env::var("CLAUDE_CODE_SESSION_ID").ok())
        .unwrap_or_else(|| die("uso: ctx-left [<uuid>|--all]   (default: $CLAUDE_CODE_SESSION_ID)", 2));

    match conn.query_row(SQL_ONE, [&sid], row_to_line) {
        Ok(line) => println!("{line}"),
        Err(_) => die(&format!("nessun dato per la sessione {sid}"), 1),
    }
}
