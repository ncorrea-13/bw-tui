# bw-tui

A terminal UI for [Bitwarden](https://bitwarden.com/), written in Rust with [ratatui](https://ratatui.rs/).

This is a personal project that I use and maintain for my own setup (Sway on Wayland, self-hosted Bitwarden server). It does **not** talk to the Bitwarden API or handle any crypto by itself — it just drives the official [`bw` CLI](https://bitwarden.com/help/cli/) and wraps it in a nicer, more complete interface than a shell script + `fzf` can give you.

I'm not affiliated with Bitwarden. Use it at your own risk, and read the code before you trust it with your vault.

## Why this exists

I used to have a small bash script (still kept in `reference/bw-tui.sh`) that unlocked the vault, listed items through `fzf`, and copied the selected password to the clipboard. It worked, but it could only do one thing. I wanted something that could also log in from scratch, talk to my self-hosted server, sync, browse folders, and generate passwords, without turning the bash script into an unreadable mess. So I rewrote it in Rust.

## Features

- **Full session flow**: it checks `bw status` on startup and shows the right screen — server config if you're not logged in at all, email + password (+ 2FA) if you need to log in, or just the master password if the CLI is already authenticated but locked. All of that runs on a background thread, so the UI never freezes while `bw` is doing its thing — you get a spinner instead.
- **Session cache compatible with the old script**: it reuses `~/.cache/bw-tui/session`, so if you already unlocked the vault with the bash version, it picks that session up instead of asking again.
- **Popup-friendly Vault tab**: a single full-width item list by default (this is meant to be run as a compact popup, not a fullscreen app). Folders collapse into a hidden top bar (`f` to show it) and item detail opens as a centered popup (`Enter`) instead of eating a permanent column.
- **Vim-style keys** in the vault list: `j`/`k` to move, `gg`/`G` to jump to the top/bottom, `h`/`l` to switch folders, `/` to search (like in vim, not a fzf-style always-on filter).
- **Generator tab**: wraps `bw generate` with length and character-set options.
- **Account tab**: shows the server, account email and last sync time, and lets you sync, lock or log out.
- **Clipboard handling**: copies through `wl-copy` and clears the clipboard (and `cliphist` history) a few seconds after copying a password, username, TOTP code, card number, or note.
- **Config file**: reads `~/.config/bw-tui/config.json`, creating it with defaults on first run instead of hardcoding things like the `bw` command, session timeout, or clipboard-clear delay. See [Configuration](#configuration).

## Requirements

This was built for a Wayland setup and currently only works there. You'll need:

- The [`bw` CLI](https://bitwarden.com/help/cli/) installed and available on your `PATH`.
- `wl-copy` / `wl-paste` (from `wl-clipboard`).
- `cliphist` (used to scrub the copied secret from your clipboard history).
- `notify-send` (from `libnotify`), for the small desktop notifications.
- A Rust toolchain (stable) if you want to build it yourself.

## Build

```sh
cargo build --release
./target/release/bw-tui
```

## Configuration

On first run (Rust binary or `reference/bw-tui.sh`, whichever you launch first) it creates `~/.config/bw-tui/config.json` — or `$XDG_CONFIG_HOME/bw-tui/config.json` if that's set — with these defaults:

```json
{
  "bw_cmd": "bw",
  "session_max_age_secs": 1200,
  "clipboard_clear_secs": 9,
  "generator": {
    "length": 20,
    "uppercase": true,
    "lowercase": true,
    "numbers": true,
    "special": false
  }
}
```

- `bw_cmd`: how to invoke the Bitwarden CLI. It's word-split before running, so it can be a wrapper too, e.g. `"flatpak run --command=bw com.bitwarden.desktop"`.
- `session_max_age_secs`: how long a cached session is considered valid before you're asked to unlock again (both the Rust app and the bash script use this for their own auto-lock timer).
- `clipboard_clear_secs`: how long a copied secret stays on the clipboard before it's wiped.
- `generator`: the Generator tab's starting options; you can still change them per-session from the tab itself.

The file is plain JSON on purpose: the Rust side already depends on `serde_json`, and the shell script already depends on `jq` for parsing `bw`'s output, so neither side needed a new dependency to read or write it. Edit the file directly — there's no in-app settings screen.

## Keybindings

**Vault tab** (normal mode)

| Key                | Action                                     |
| ------------------ | ------------------------------------------ |
| `j` / `k` or ↓ / ↑ | move selection                             |
| `gg` / `G`         | jump to top / bottom                       |
| `f`                | show/hide the folder bar                   |
| `h` / `l`          | previous / next folder                     |
| `/`                | enter search mode                          |
| `Enter`            | open the item detail popup                 |
| `R` or `F5`        | refresh the item list                      |
| `q` / `Esc`        | quit (`Esc` clears an active filter first) |

**Item detail popup** (after pressing `Enter` on an item): shows everything about it — for a login, that's username/URL/TOTP plus the password masked; for a card, cardholder/brand/expiry plus the number masked; for a note, its full text.

| Key     | Action                                                           |
| ------- | ---------------------------------------------------------------- |
| `Enter` | copy the item's "primary" secret (password / card number / note) |
| `u`     | copy username (logins)                                           |
| `t`     | copy TOTP code (logins)                                          |
| `r`     | reveal password or card number                                   |
| `n`     | copy notes, if the item has any                                  |
| `Esc`   | close the popup                                                  |

**Search mode** (after pressing `/`): type to filter, `Enter` confirms and goes back to normal mode, `Esc` cancels and clears the filter.

**Generator tab**: `u` / `l` / `n` / `s` toggle character sets, `↑`/`↓` change the length, `Enter` generates, `c` copies the result.

**Account tab**: `s` syncs, `l` locks the vault, `o` logs out (asks for confirmation).

**Anywhere in the main screen**: `Tab` / `Shift+Tab` switch between the Vault, Generator and Account tabs.

## Testing

There's no real automated test suite for the UI itself (a TUI needs a terminal, and the vault operations need a real `bw` session), but:

- `cargo test` runs unit tests for the pure state logic — vim motions, search mode, folder filtering — using fake in-memory items, no `bw` calls involved.
- `scripts/smoke_test.py` drives the compiled binary inside a real pseudo-terminal and dumps what it renders, so you can check that a screen shows up correctly and that error paths don't panic. Read the warning at the top of the script before running it — it talks to your real `bw` CLI.

## Status

Actively maintained, but built for my own use first. Things I might add later: SSO / API key login, item creation and editing, attachments. Issues and PRs are welcome if you find this useful too.
