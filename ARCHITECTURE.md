# Architecture

This file covers how the project is put together. For install/usage instructions, see [`README.md`](./README.md).

`bw-tui` is not a Bitwarden client. Both versions described below are, at their core, thin wrappers around the official Bitwarden CLI.

## Two versions

There are two ways to use this project, in two different folders:

| Version  | What it is                                                                                                                     |
| -------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **Rust** | The full version. A Terminal User Interface with folders, item detail, password generator, account tab, config file, and more. |
| **Bash** | One little script that unlocks the vault, lists items, and copies what you pick. It does one thing, and it does it fast.       |

I started with the bash script. Later I wanted something that could also log in from scratch, sync, browse folders, and generate passwords. So I wrote the Rust version. The bash script is still useful if you just want a quick picker and don't want to build anything.

The two versions are independent. They're built to be interchangeable day to day. They agree on the same on-disk contract:

- **Config**: both read `~/.config/bw-tui/config.json` with the same keys (see [Configuration](./README.md#configuration)). The same values are used for both versions.
- **Session cache**: both read/write `~/.cache/bw-tui/session` and `~/.cache/bw-tui/session_time`. Whichever version you used last to unlock the vault, the other one will pick up that same session instead of asking for the master password again.

## Clipboard backends

At startup, both check `WSL_DISTRO_NAME`/`WSL_INTEROP`, and fall back to checking `/proc/version` for `microsoft`. That decides which backend is used every time something gets copied:

- **Native Wayland**: a background job removes the secret from clipboard history and clears the clipboard, notifying when it does.
- **WSL2**: the same as the wayland version but it does not notify, because WSL2 has no native notification system.

## Session auto-lock

Both versions relock the vault on their own once a session gets too old, but they get there differently, because of how long each process actually lives:

- The **Rust version** is a long-running process with its own event loop. It just checks on every tick while the main screen is showing, and relocks in place if it's expired.
- The **bash script** it spawns a detached background subshell that sleeps and then locks the vault and clears the cache files, but only if no newer session was created in the meantime.

## Testing strategy

There's no real automated test suite for the UI itself, but:

- `cargo test` runs unit tests for the plain state logic, vim motions, search mode, folder filtering, using fake in-memory items, with no `bw` calls involved.
- `scripts/smoke_test.py` runs the compiled binary inside a real pseudo-terminal and dumps what it draws, so you can check that a screen shows up correctly and that error paths don't crash. It talks to your real `bw` CLI.

The bash script has no automated tests; it's short enough to read in full and verify by hand.
