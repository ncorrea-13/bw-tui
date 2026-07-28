# bw-tui

A terminal UI for [Bitwarden](https://bitwarden.com/). It just drives the official [`bw` CLI](https://bitwarden.com/help/cli/) and gives it an interface.

This is a personal project. I'm not affiliated with Bitwarden in any way. Use it at your own risk.

## Two versions

There are two version for this project: The Rust TUI, and a lite bash script in [`bash/`](#bash-version-lite). Both talk to the same `bw` CLI and share the same config file and cached session, so you can mix them. See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for why both exist.

## Dependencies

| Program          | Purpose                 | Needed for              | Platform      |
| ---------------- | ----------------------- | ----------------------- | ------------- |
| `bw` CLI         | talks to Bitwarden      | Bash and Rust           | Both          |
| `Rust toolchain` | compiling program       | Rust (only for compile) | Both          |
| `fzf`            | item picker             | Bash                    | Both          |
| `jq`             | parse config/`bw` JSON  | Bash                    | Both          |
| `wl-clipboard`   | clipboard               | Bash and Rust           | Linux Wayland |
| `cliphist`       | clear clipboard history | Bash and Rust           | Both          |
| `libnotify`      | desktop notifications   | Bash and Rust           | Linux Wayland |
| `clip.exe`       | clipboard               | Bash and Rust           | WSL2          |
| A Nerd Font      | UI icons                | Bash and Rust           | Both          |

---

## Rust version

### Install

You can download a prebuilt binary from the [Releases](https://github.com/ncorrea-13/bw-tui/releases/latest)
Also you can build it yourself:

```sh
git clone https://github.com/ncorrea-13/bw-tui
cd bw-tui
cargo build --release
cp target/release/bw-tui ~/.local/bin/bw-tui
```

### Features

- **Full session flow**: on startup it checks `bw status` and shows the right screen: server setup if you're not logged in at all, email + password (+ 2FA) if you need to log in, or just the master password if the CLI is already logged in but locked. This runs on a background thread, so the UI doesn't freeze while `bw` is working, you get a spinner instead.
- **Session cache compatible with the bash version**: it reuses `~/.cache/bw-tui/session`, so if you already unlocked the vault with the bash script, it picks up that session instead of asking again.
- **Popup-friendly Vault tab**: one full-width item list by default. Folders show in a top bar and item detail opens as a popup (`Enter`) instead of taking a whole column.
- **Create and edit items**: logins, secure notes, cards, and identities, without leaving the Vault tab (`n` to create, `e` on the detail popup to edit). The form has its own password generator (`Ctrl+G`) sharing the Generator tab's settings, and `Ctrl+R` to pull up a login's current password while editing.
- **Vim-style keys** in the vault list: `j`/`k` to move, `gg`/`G` to jump to top/bottom, `h`/`l` to switch folders, `/` to search.
- **Generator tab**: wraps `bw generate` with length and character-set options.
- **Account tab**: shows the server, account email and last sync time, and lets you sync, lock or log out.
- **Clipboard handling**: it detects at startup if it's running under WSL or a native Linux/Wayland host, and picks the right way to copy things. See [Clipboard backends](./ARCHITECTURE.md#clipboard-backends) in ARCHITECTURE.md.
- **Config file**: reads `~/.config/bw-tui/config.json` and creates it with defaults on first run, instead of hardcoding things like the `bw` command, session timeout, or clipboard-clear delay. See [Configuration](#configuration).

Built first for a Wayland setup, and it also runs under WSL2. See [Dependencies](#dependencies).

### Keybindings

See [`KEYBINDINGS.md`](./KEYBINDINGS.md) for the full reference (Vault tab, item detail popup, search mode, Generator tab, Account tab).

### Testing

See [Testing strategy](./ARCHITECTURE.md#testing-strategy) in ARCHITECTURE.md.

---

## Bash version (lite)

The script that started this project. It follows the Kiss principle and only do one thing: unlock the vault, show your items, and copy the password or the secret you picked. No tabs, no folders view, no generator, just a fast picker for logins, notes and cards.

It also starts a background job that locks the vault again once the session times out, so you don't have to remember to lock it yourself. See [Dependencies](#dependencies).

### Usage

```sh
wget 'https://github.com/ncorrea-13/bw-tui/blob/main/bash/bw-tui.sh'
chmod +x bash/bw-tui.sh
cp bw-tui.sh ~/.local/bin/bw-tui.sh
```

It's meant to be bound to a keyboard shortcut in your window manager, so it pops up, you copy something, and it closes. The same way you'd use the Rust version as a popup.

---

## Configuration

Both versions create and use the same config file: `~/.config/bw-tui/config.json` with these defaults. There's no settings screen so you have to edit it manually.

```json
{
  "bw_cmd": "bw",
  "session_max_age_secs": 1800,
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

- `bw_cmd`: how to call the Bitwarden CLI. You can define here if bw is not in your $PATH. It can also be a wrapper, e.g. `"flatpak run --command=bw com.bitwarden.desktop"`.
- `session_max_age_secs`: Time for the auto-lock background job to wait before locking the vault again.
- `clipboard_clear_secs`: How long a secret stays on the clipboard before it gets wiped.
- `generator`: the Generator tab's starting options. You can still change them per-session from the tab itself.

## Status

Actively maintained, but built for my own use first. Issues and PRs are welcome if you find this useful too.

---

## License

GPLv3 - see [LICENSE](LICENSE) for details.

_Mendoza, Argentina - Nicolás Correa ([ncorrea-13](https://github.com/ncorrea-13))_
