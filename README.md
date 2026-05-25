# cc-statusline

Fast Rust statusline for [Claude Code](https://claude.com/claude-code). Drop-in replacement for the shell/Python statusline scripts, with a ~30× faster cold-start.

![statusline screenshot](screenshots/statusline.png)

## What it shows

| Field        | Source                                       |
| ------------ | -------------------------------------------- |
| `model/effort` | `model.display_name` / `effort.level`      |
| `ctx_left`   | `context_window.remaining_percentage`        |
| `cwd`        | `cwd` (with `$HOME` folded to `~`)           |
| `week_left`  | `100 - rate_limits.seven_day.used_percentage`|
| `session_id` | `session_id`                                 |

## Build

```bash
git clone https://github.com/efebia-com/cc-statusline.git
cd cc-statusline
cargo build --release
```

The binary lands at `target/release/statusline`.

## Configure

Add this to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/absolute/path/to/cc-statusline/target/release/statusline"
  }
}
```

Restart Claude Code (or start a new session). Done.

## Input format

Claude Code pipes a JSON payload to the statusline's stdin and reads the rendered line from stdout. See [`examples/sample-input.json`](examples/sample-input.json) for the full schema — it includes `model`, `workspace`, `cost`, `context_window`, `rate_limits`, and more.

To capture a live sample from your own session, add a single line to `main()`:

```rust
std::fs::write("/tmp/statusline-input.json", &buf).ok();
```

## Customize

The whole statusline is ~50 lines. Edit `src/main.rs` to change:

- **Layout / which fields appear** — the `fields` array near the bottom of `main()`
- **Color palette** — the `colorize` function at the top
- **Color thresholds** — currently flat per-field; add `match` arms as needed

Rebuild with `cargo build --release` after edits.

## License

MIT — see [LICENSE](LICENSE).
