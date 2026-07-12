# bw-tui

A terminal UI for [Bitwarden](https://bitwarden.com/). It does **not** talk to the Bitwarden API or handle any crypto itself. It just drives the official [`bw` CLI](https://bitwarden.com/help/cli/) and gives it a nicer interface than a shell script with `fzf`.

This is a personal project. I built it and use it for my own setup (Sway on Wayland, self-hosted Bitwarden server). I'm not affiliated with Bitwarden in any way. Use it at your own risk, and read the code before you trust it with your vault.

## Two versions

There are two ways to use this project, in two different folders:

| Version  | Folder                        | What it is                                                                                                                                                |
| -------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rust** | [`src/`](#rust-version-full)  | The full version. A real TUI made with [ratatui](https://ratatui.rs/), with folders, item detail, password generator, account tab, config file, and more. |
| **Bash** | [`bash/`](#bash-version-lite) | A lite version. One script that unlocks the vault, lists items with `fzf`, and copies what you pick. It does one thing, and it does it fast.              |

I started with the bash script (it's still there, working). Later I wanted something that could also log in from scratch, sync, browse folders, and generate passwords, without turning the script into something hard to read. So I wrote the Rust version. The bash script is still useful if you just want a quick picker and don't want to build anything, so I kept both.

Both versions read the same config file and can share the same cached session (see [Configuration](#configuration) below), so you can mix them if you want.

---

## Rust version (full)

### Features

- **Full session flow**: on startup it checks `bw status` and shows the right screen: server setup if you're not logged in at all, email + password (+ 2FA) if you need to log in, or just the master password if the CLI is already logged in but locked. This runs on a background thread, so the UI doesn't freeze while `bw` is working, you get a spinner instead.
- **Session cache compatible with the bash version**: it reuses `~/.cache/bw-tui/session`, so if you already unlocked the vault with the bash script, it picks up that session instead of asking again.
- **Popup-friendly Vault tab**: one full-width item list by default. Folders are hidden in a top bar and item detail opens as a popup (`Enter`) instead of taking a whole column.
- **Vim-style keys** in the vault list: `j`/`k` to move, `gg`/`G` to jump to top/bottom, `h`/`l` to switch folders, `/` to search.
- **Generator tab**: wraps `bw generate` with length and character-set options.
- **Account tab**: shows the server, account email and last sync time, and lets you sync, lock or log out.
- **Clipboard handling**: it detects at startup if it's running under WSL or a native Linux/Wayland host, and picks the right way to copy things. See [Clipboard backends](#clipboard-backends).
- **Config file**: reads `~/.config/bw-tui/config.json` and creates it with defaults on first run, instead of hardcoding things like the `bw` command, session timeout, or clipboard-clear delay. See [Configuration](#configuration).

### Requirements

Built first for a Wayland setup, and it also runs under WSL2. You need:

- The [`bw` CLI](https://bitwarden.com/help/cli/) installed and on your `PATH` (inside WSL, if that's where you run it).
- A Rust toolchain to build it.

On native Wayland, you also need:

- `wl-copy` / `wl-paste` (`wl-clipboard`).
- `cliphist` (to clean clipboard history).
- `notify-send` (`libnotify`), for the small desktop notifications.

On WSL2, you also need:

- Windows interop enabled, so `clip.exe` can be reached from `PATH`.

### Clipboard backends

At startup, `clipboard::is_wsl()` checks `WSL_DISTRO_NAME`/`WSL_INTEROP`, and falls back to checking `/proc/version` for `microsoft`. That decides which backend is used every time you copy something:

- **Native Wayland**: copies with `wl-copy`, and a background thread removes the secret from `cliphist` history and clears the clipboard after `clipboard_clear_secs`, sending a `notify-send` message when it does.
- **WSL2**: copies with `clip.exe`, straight to the Windows clipboard. Autoclear does nothing here on purpose, polling the Windows clipboard would mean calling `clip.exe`/`powershell.exe` again and again across the WSL/Windows interop boundary, which is slow. So the WSL path stays simple and leaves the secret on the clipboard until you overwrite it yourself. The status line reflects this.

### Build

```sh
cargo build --release
./target/release/bw-tui
```

### Keybindings

**Vault tab** (normal mode)

| Key                | Action                     |
| ------------------ | -------------------------- |
| `j` / `k` or ↓ / ↑ | move selection             |
| `gg` / `G`         | jump to top / bottom       |
| `f`                | show/hide the folder bar   |
| `h` / `l`          | previous / next folder     |
| `/`                | enter search mode          |
| `Enter`            | open the item detail popup |
| `R` or `F5`        | refresh the item list      |
| `q` / `Esc`        | quit                       |

**Item detail popup**: shows everything about it for a login, that's username/URL/TOTP plus the password; for a card, cardholder/brand/expiry plus the number; for a note, its full text.

| Key     | Action                                                        |
| ------- | ------------------------------------------------------------- |
| `Enter` | copy the item's "main" secret (password / card number / note) |
| `u`     | copy username (logins)                                        |
| `t`     | copy TOTP code (logins)                                       |
| `r`     | reveal password or card number                                |
| `n`     | copy notes, if the item has any                               |
| `Esc`   | close the popup                                               |

**Search mode**: type to filter, `Enter` confirms and goes back to normal mode, `Esc` cancels and clears the filter.

**Generator tab**: `u` / `l` / `n` / `s` turn character sets on/off, `↑`/`↓` change the length, `Enter` generates, `c` copies the result.

**Account tab**: `s` syncs, `l` locks the vault, `o` logs out (asks for confirmation).

**Anywhere in the main screen**: `Tab` / `Shift+Tab` switch between the Vault, Generator and Account tabs.

### Testing

There's no real automated test suite for the UI itself, but:

- `cargo test` runs unit tests for the plain state logic, vim motions, search mode, folder filtering, using fake in-memory items, no `bw` calls involved.
- `scripts/smoke_test.py` runs the compiled binary inside a real pseudo-terminal and dumps what it draws, so you can check that a screen shows up correctly and that error paths don't crash. Read the warning at the top of the script before you run it. It talks to your real `bw` CLI.

---

## Bash version (lite)

The script that started this project. It's in [`bash/bw-tui.sh`](bash/bw-tui.sh) and it does one thing: unlock the vault, show your items in `fzf`, and copy what you picked. No tabs, no folders view, no generator, just a fast picker for logins, notes and cards.

What it does, step by step:

1. Reads/creates the same `config.json` the Rust version uses.
2. Checks if there's a cached session under `~/.cache/bw-tui/` and if it's still fresh. If not, it asks for your master password and unlocks the vault.
3. Lists your items with `bw list items` and shows them in `fzf`.
4. Depending on the item type:
   - **Login** → copies the password.
   - **Note** → copies the note text.
   - **Card** → asks if you want the number or the CVV, then copies it.
5. On native Wayland, it copies with `wl-copy`, waits `clipboard_clear_secs`, then clears the clipboard and removes the value from `cliphist` history. On WSL2, it copies with `clip.exe` and does not auto-clear. See [Clipboard backends](#clipboard-backends).

It also starts a background job that locks the vault again once the session times out, so you don't have to remember to lock it yourself.

### Requirements

- The [`bw` CLI](https://bitwarden.com/help/cli/), on your `PATH`.
- `jq`, to read the config file and parse `bw`'s JSON output.
- `fzf`, for the item picker.
- On native Wayland: `wl-copy` / `wl-paste`, `cliphist`, `notify-send`.
- On WSL2: Windows interop enabled, so `clip.exe` is on `PATH`.

### Usage

```sh
chmod +x bash/bw-tui.sh
./bash/bw-tui.sh
```

It's meant to be bound to a keyboard shortcut in your window manager, so it pops up, you copy something, and it closes. The same way you'd use the Rust version as a popup.

---

## Configuration

Both versions share the same config file: `~/.config/bw-tui/config.json` (or `$XDG_CONFIG_HOME/bw-tui/config.json` if you set that variable). Whichever version you run first creates it with these defaults:

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

- `bw_cmd`: how to call the Bitwarden CLI. It's split into words before running, so it can also be a wrapper, e.g. `"flatpak run --command=bw com.bitwarden.desktop"`.
- `session_max_age_secs`: how long a cached session stays valid before you have to unlock again (for auto-lock).
- `clipboard_clear_secs`: how long a copied secret stays on the clipboard before it gets wiped.
- `generator`: the Generator tab's starting options (Rust version only). You can still change them per-session from the tab itself.

The file is plain JSON on purpose: the Rust side already depends on `serde_json`, and the bash script already depends on `jq`, so neither side needed a new dependency just to read or write it. Edit the file directly. There's no settings screen.

## Status

Actively maintained, but built for my own use first. Things I might add later: SSO / API key login, item creation and editing, attachments (Rust version). Issues and PRs are welcome if you find this useful too.
