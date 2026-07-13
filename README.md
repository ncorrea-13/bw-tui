# bw-tui

A terminal UI for [Bitwarden](https://bitwarden.com/). It does **not** talk to the Bitwarden API or handle any crypto itself. It just drives the official [`bw` CLI](https://bitwarden.com/help/cli/) and gives it a nicer interface than a shell script with `fzf`.

This is a personal project. I built it and use it for my own setup (Sway on Wayland, self-hosted Bitwarden server). I'm not affiliated with Bitwarden in any way. Use it at your own risk, and read the code before you trust it with your vault.

## Two versions

There are two ways to use this project, in two different folders: a full Rust TUI in [`src/`](#rust-version-full), and a lite bash script in [`bash/`](#bash-version-lite). Both talk to the same `bw` CLI and share the same config file and cached session, so you can mix them. See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for why both exist, how they relate, and design notes on the clipboard handling and session auto-lock.

## Dependencies

| Program                                  | Purpose                 | Needed for     |
| ----------------------------------------- | ------------------------ | --------------- |
| [`bw` CLI](https://bitwarden.com/help/cli/) | talks to Bitwarden      | both versions   |
| Rust toolchain                            | build                    | Rust version    |
| `fzf`                                     | item picker               | Bash version    |
| `jq`                                      | parse config/`bw` JSON    | Bash version    |
| `wl-copy` / `wl-paste` (`wl-clipboard`)   | clipboard                 | native Wayland  |
| `cliphist`                                | clear clipboard history   | native Wayland  |
| `notify-send` (`libnotify`)               | desktop notifications     | native Wayland  |
| Windows interop (`clip.exe` on `PATH`)    | clipboard                 | WSL2            |

---

## Rust version (full)

### Features

- **Full session flow**: on startup it checks `bw status` and shows the right screen: server setup if you're not logged in at all, email + password (+ 2FA) if you need to log in, or just the master password if the CLI is already logged in but locked. This runs on a background thread, so the UI doesn't freeze while `bw` is working, you get a spinner instead.
- **Session cache compatible with the bash version**: it reuses `~/.cache/bw-tui/session`, so if you already unlocked the vault with the bash script, it picks up that session instead of asking again.
- **Popup-friendly Vault tab**: one full-width item list by default. Folders are hidden in a top bar and item detail opens as a popup (`Enter`) instead of taking a whole column.
- **Vim-style keys** in the vault list: `j`/`k` to move, `gg`/`G` to jump to top/bottom, `h`/`l` to switch folders, `/` to search.
- **Generator tab**: wraps `bw generate` with length and character-set options.
- **Account tab**: shows the server, account email and last sync time, and lets you sync, lock or log out.
- **Clipboard handling**: it detects at startup if it's running under WSL or a native Linux/Wayland host, and picks the right way to copy things. See [Clipboard backends](./ARCHITECTURE.md#clipboard-backends) in ARCHITECTURE.md.
- **Config file**: reads `~/.config/bw-tui/config.json` and creates it with defaults on first run, instead of hardcoding things like the `bw` command, session timeout, or clipboard-clear delay. See [Configuration](#configuration).

Built first for a Wayland setup, and it also runs under WSL2. See [Dependencies](#dependencies).

### Build

```sh
cargo build --release
./target/release/bw-tui
```

### Keybindings

See [`KEYBINDINGS.md`](./KEYBINDINGS.md) for the full reference (Vault tab, item detail popup, search mode, Generator tab, Account tab).

### Testing

See [Testing strategy](./ARCHITECTURE.md#testing-strategy) in ARCHITECTURE.md.

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
5. Copies it to the clipboard. On native Wayland it auto-clears after `clipboard_clear_secs`; on WSL2 it doesn't. See [Clipboard backends](./ARCHITECTURE.md#clipboard-backends) in ARCHITECTURE.md.

It also starts a background job that locks the vault again once the session times out, so you don't have to remember to lock it yourself. See [Dependencies](#dependencies).

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

The file is plain JSON on purpose: both sides already parse JSON elsewhere, so it added no new dependency. Edit it directly; there's no settings screen.

## Status

Actively maintained, but built for my own use first. Things I might add later: SSO / API key login, item creation and editing, attachments (Rust version). Issues and PRs are welcome if you find this useful too.

---

## License

MIT License - see [LICENSE](LICENSE) for details.

_Mendoza, Argentina - Nicolás Correa ([ncorrea-13](https://github.com/ncorrea-13))_
