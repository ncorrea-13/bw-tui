# Architecture

This file covers how the project is put together: why there are two
versions, how they relate, and the design decisions behind the parts that
aren't obvious from just using the tool. For install/usage instructions,
see [`README.md`](./README.md).

`bw-tui` is not a Bitwarden client: it does **not** talk to the Bitwarden
API or handle any crypto itself. Both versions described below are, at
their core, thin wrappers around the official [`bw` CLI](https://bitwarden.com/help/cli/):
they shell out to it, read its (mostly JSON) output, and present it more
conveniently than typing the equivalent `bw` commands by hand.

## Two versions

There are two ways to use this project, in two different folders:

| Version  | Folder    | What it is                                                                                                                                                |
| -------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Rust** | [`src/`](./README.md#rust-version-full)  | The full version. A real TUI made with [ratatui](https://ratatui.rs/), with folders, item detail, password generator, account tab, config file, and more. |
| **Bash** | [`bash/`](./README.md#bash-version-lite) | A lite version. One script that unlocks the vault, lists items with `fzf`, and copies what you pick. It does one thing, and it does it fast.              |

I started with the bash script (it's still there, working). Later I wanted
something that could also log in from scratch, sync, browse folders, and
generate passwords, without turning the script into something hard to
read. So I wrote the Rust version. The bash script is still useful if you
just want a quick picker and don't want to build anything, so I kept both.

The two versions are independent code — the bash script doesn't call the
Rust binary, and there's no shared library between them — but they're built
to be interchangeable day to day, because they agree on the same on-disk
contract:

- **Config**: both read `~/.config/bw-tui/config.json` (or
  `$XDG_CONFIG_HOME/bw-tui/config.json`), with the same keys
  (`bw_cmd`, `session_max_age_secs`, `clipboard_clear_secs`, `generator.*`
  — see [Configuration](./README.md#configuration) for what each one does).
  It's plain JSON on purpose: the Rust side already depends on
  `serde_json`, and the bash script already depends on `jq`, so neither
  side needed a new dependency just to read or write it. There's no single
  source of truth for the default values, though — they're duplicated by
  hand in `src/config.rs` (`Config::default()`) and in the heredoc at the
  top of `bash/bw-tui.sh`. If you change one, change the other.
- **Session cache**: both read/write `~/.cache/bw-tui/session` (the raw
  `bw` session key, permissions `600`) and `~/.cache/bw-tui/session_time`
  (the Unix timestamp of when it was created). Whichever version you used
  last to unlock the vault, the other one will pick up that same session
  instead of asking for the master password again, as long as it's still
  younger than `session_max_age_secs`.

That shared file contract is the only real "integration" between the two:
they're two independent programs reading and writing the same couple of
plain-text files.

## Clipboard backends

At startup, `clipboard::is_wsl()` (Rust) / `is_wsl()` (bash) checks
`WSL_DISTRO_NAME`/`WSL_INTEROP`, and falls back to checking `/proc/version`
for `microsoft`. That decides which backend is used every time something
gets copied, in both versions:

- **Native Wayland**: copies with `wl-copy`, and a background job (a thread
  in Rust, a `nohup`'d subshell loop in bash) removes the secret from
  `cliphist` history and clears the clipboard after `clipboard_clear_secs`,
  sending a `notify-send` message when it does.
- **WSL2**: copies with `clip.exe`, straight to the Windows clipboard.
  Autoclear does nothing here on purpose: polling the Windows clipboard
  would mean calling `clip.exe`/`powershell.exe` again and again across the
  WSL/Windows interop boundary, which is slow. So the WSL path stays simple
  and leaves the secret on the clipboard until you overwrite it yourself.
  The status line reflects this.

## Session auto-lock

Both versions relock the vault on their own once a session gets too old
(`session_max_age_secs`), but they get there differently, because of how
long each process actually lives:

- The **Rust version** is a long-running process with its own event loop.
  It just checks `session_age() > session_max_age_secs` on every tick
  (every ~250ms) while the main screen is showing, and relocks in place if
  it's expired.
- The **bash script** exits right after it copies something, so it can't
  watch its own session while "running" — there's nothing running anymore.
  Instead, right after unlocking, it spawns a detached (`nohup`) background
  subshell that sleeps for `session_max_age_secs` and then locks the vault
  and clears the cache files, but only if no newer session was created in
  the meantime (it checks that the session timestamp file still matches the
  one it captured when it started sleeping). This avoids the need to track
  and kill a previous timer when a new session replaces an old one — old
  timers just no-op if they wake up to find a fresher session already in
  place.

## Testing strategy

There's no real automated test suite for the UI itself, but:

- `cargo test` runs unit tests for the plain state logic — vim motions,
  search mode, folder filtering — using fake in-memory items, with no `bw`
  calls involved. This works because keyboard handling only ever calls
  methods on the app state, never `bw` directly; all the I/O lives behind
  `src/bw.rs`, safely out of reach of these tests.
- `scripts/smoke_test.py` runs the compiled binary inside a real
  pseudo-terminal and dumps what it draws, so you can check that a screen
  shows up correctly and that error paths don't crash. Read the warning at
  the top of the script before you run it — it talks to your real `bw`
  CLI.

The bash script has no automated tests; it's short enough to read in full
and verify by hand.
