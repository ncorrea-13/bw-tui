# Keybindings (Rust version)

Full keyboard reference for the Rust TUI (`src/`). The bash script has no keybindings of its own. It hands off navigation to `fzf`, whose keys are `fzf`'s own (arrows/Ctrl-J/Ctrl-K to move, type to filter, Enter to select, Esc/Ctrl-C to cancel).

**Anywhere in the main screen**: `Tab` / `Shift+Tab` switch between the Vault, Generator and Account tabs. `Ctrl+C` quits from anywhere, including the login/unlock screens.

## Vault tab (normal mode)

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

## Item detail popup

Shows everything about the selected item: for a login, that's username/URL/TOTP plus the password; for a card, cardholder/brand/expiry plus the number; for a note, its full text.

| Key     | Action                                                        |
| ------- | ------------------------------------------------------------- |
| `Enter` | copy the item's "main" secret (password / card number / note) |
| `u`     | copy username (logins)                                        |
| `t`     | copy TOTP code (logins)                                       |
| `r`     | reveal password or card number                                |
| `n`     | copy notes, if the item has any                               |
| `Esc`   | close the popup                                               |

## Search mode

Type to filter, `Enter` confirms and goes back to normal mode, `Esc` cancels and clears the filter.

## Generator tab

| Key       | Action                    |
| --------- | ------------------------- |
| `u`       | toggle uppercase          |
| `l`       | toggle lowercase          |
| `n`       | toggle numbers            |
| `s`       | toggle special characters |
| `↑` / `↓` | change length             |
| `Enter`   | generate                  |
| `c`       | copy the generated result |

## Account tab

| Key | Action                          |
| --- | ------------------------------- |
| `s` | sync                            |
| `l` | lock the vault                  |
| `o` | log out (asks for confirmation) |
