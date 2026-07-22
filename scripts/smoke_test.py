#!/usr/bin/env python3
"""
PTY-based smoke test harness for bw-tui.

Renders the TUI inside a real pseudo-terminal (ratatui/crossterm need one)
and dumps the ANSI-stripped output, so screens can be checked without a
human at the keyboard. Useful for verifying that a screen renders and that
error paths surface correctly without panicking.

SAFETY: this drives a real `bw` CLI subprocess. Never point it at an
already-unlocked vault -- it will dump real vault contents (item names,
usernames, folders) to stdout. Run `bw lock` first if unsure. Never pass a
real master password via --key (it ends up in your shell history); use a
throwaway/invalid password when testing the unlock error path.

Usage:
  python3 scripts/smoke_test.py [--wait SECONDS] [--rows N] [--cols N]
      [--key DELAY:TEXT]... -- <path-to-binary>

  --key DELAY:TEXT   after DELAY seconds (float), send TEXT as keystrokes.
                      TEXT supports \\r for Enter and \\x1b for Esc.
                      Repeatable, sent in order.

Examples:
  # Just render whatever the first screen is
  python3 scripts/smoke_test.py -- ./target/release/bw-tui

  # Exercise the wrong-password error path on the Unlock screen
  python3 scripts/smoke_test.py --wait 10 \\
      --key 5:not-the-real-password --key 0.5:\\r -- ./target/release/bw-tui
"""

import argparse
import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time


def strip_ansi(text: str) -> str:
    text = re.sub(r"\x1b\[[0-9;?]*[a-zA-Z]", "", text)
    text = re.sub(r"\x1b\][^\x07]*\x07", "", text)
    return text


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--wait",
        type=float,
        default=6.0,
        help="total seconds to capture output for (default: 6)",
    )
    parser.add_argument("--rows", type=int, default=30)
    parser.add_argument("--cols", type=int, default=100)
    parser.add_argument(
        "--key",
        action="append",
        default=[],
        metavar="DELAY:TEXT",
        help="after DELAY seconds, send TEXT as keystrokes (repeatable)",
    )
    parser.add_argument(
        "binary",
        nargs=argparse.REMAINDER,
        help="binary and args to run, e.g. -- ./target/release/bw-tui",
    )
    args = parser.parse_args()

    cmd = [a for a in args.binary if a != "--"]
    if not cmd:
        parser.error("no binary given, e.g.: smoke_test.py -- ./target/release/bw-tui")

    master, slave = pty.openpty()
    fcntl.ioctl(
        slave, termios.TIOCSWINSZ, struct.pack("HHHH", args.rows, args.cols, 0, 0)
    )

    proc = subprocess.Popen(
        cmd, stdin=slave, stdout=slave, stderr=slave, close_fds=True
    )
    os.close(slave)

    data = b""

    def pump(duration: float) -> None:
        nonlocal data
        end = time.time() + duration
        while time.time() < end:
            r, _, _ = select.select([master], [], [], 0.1)
            if master in r:
                try:
                    chunk = os.read(master, 65536)
                except OSError:
                    return
                if not chunk:
                    return
                data += chunk

    steps = []
    for raw in args.key:
        delay_str, _, payload = raw.partition(":")
        steps.append(
            (
                float(delay_str),
                payload.encode().replace(b"\\r", b"\r").replace(b"\\x1b", b"\x1b"),
            )
        )

    remaining = args.wait
    for delay, payload in steps:
        pump(delay)
        remaining -= delay
        os.write(master, payload)
    pump(max(remaining, 0.5))

    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()

    print(strip_ansi(data.decode("utf-8", errors="replace")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
