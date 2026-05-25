//! Claude Code statusline: verbose `label → value` layout with rainbow value colors.

use serde_json::Value;
use std::io::{self, Read};

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

fn main() {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).expect("read stdin");
    let data: Value = serde_json::from_str(&buf).expect("parse JSON");

    let model_name   = data["model"]["display_name"].as_str().unwrap_or("?");
    let cwd          = data["cwd"].as_str().unwrap_or("?");
    let session_id   = data["session_id"].as_str().unwrap_or("?");
    let context_pct  = data["context_window"]["remaining_percentage"].as_f64().unwrap_or(100.0);
    let effort_level = data["effort"]["level"].as_str().unwrap_or("?");
    let rate_7d      = data["rate_limits"]["seven_day"]["used_percentage"].as_f64().unwrap_or(0.0);

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
    let fields = [
        format!("{} {arrow} {} {slash} {}", label("model/effort"), colorize("red", model_name), colorize("orange", effort_level)),
        format!("{} {arrow} {}", label("ctx_left"),   colorize("yellow", &format!("{context_pct:.1}%"))),
        format!("{} {arrow} {}", label("cwd"),        colorize("green",  &cwd_display)),
        format!("{} {arrow} {}", label("week_left"),  colorize("blue",   &format!("{:.0}%", 100.0 - rate_7d))),
        format!("{} {arrow} {}", label("session_id"), colorize("violet", session_id)),
    ];

    let joiner = format!(" {sep} ");
    print!("{}", fields.join(&joiner));
}
