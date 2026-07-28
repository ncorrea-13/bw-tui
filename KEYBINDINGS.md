# Keybindings

Full keyboard reference for the Rust version.

`Tab` / `Shift+Tab` switch between the Vault, Generator and Account tabs. `Ctrl+C` and `q` quits from anywhere, including the login/unlock screens.

## Vault tab

| Key                | Action                     |
| ------------------ | -------------------------- |
| `j` / `k` or ↓ / ↑ | move selection             |
| `gg` / `G`         | jump to top / bottom       |
| `h` / `l`          | previous / next folder     |
| `/`                | enter search mode          |
| `Enter`            | open the item detail popup |
| `n`                | create a new item          |
| `R` or `F5`        | refresh the item list      |
| `q` / `Esc`        | quit                       |

## Item detail popup

Shows everything about the selected item: for a login, that's username/URL/TOTP plus the password; for a card, cardholder/brand/expiry plus the number; for a note, its full text.

| Key     | Action                                                        |
| ------- | ------------------------------------------------------------- |
| `Enter` | copy the item's "main" secret (password / card number / note) |
| `u`     | copy username (logins)                                        |
| `t`     | copy TOTP code (logins)                                       |
| `r`     | reveal secret                                                 |
| `n`     | copy notes                                                    |
| `e`     | edit item                                                     |
| `Esc`   | close the popup                                               |

## Create / edit item form

Opened with `n` from the vault list, or `e` from the item detail popup. Which fields show up depends on the item type; you can't change an existing item's type, only pick one while creating.

| Key                 | Action                                     |
| ------------------- | ------------------------------------------ |
| `Tab` / `Shift+Tab` | move to the next / previous field          |
| `Ctrl+T`            | cycle item type                            |
| `Ctrl+G`            | open the password generator (login only)   |
| `Ctrl+R`            | fetch and show the item's current password |
| `Enter`             | save                                       |
| `Esc`               | closes the form                            |

Leaving the password field empty while editing keeps the item's existing password; it's never overwritten with a blank one.

### Password generator (`Ctrl+G`)

Takes over the whole screen and shares the same options as the Generator tab.

| Key             | Action                                                      |
| --------------- | ----------------------------------------------------------- |
| `u`/`l`/`n`/`s` | toggle uppercase / lowercase / numbers / special characters |
| `↑` / `↓`       | change length                                               |
| `Enter`         | generate or use the shown password                          |
| `g`             | regenerate                                                  |
| `Esc`           | back to the form without changing it                        |

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
