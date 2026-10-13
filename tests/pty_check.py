#!/usr/bin/env python3
"""Run sl-neo in a pseudo-terminal and check what only a real terminal shows:
the terminal is restored on every way out, Ctrl+C is ignored unless -e, and
termination signals end the run with 128 + signal after restoring.

Usage: python3 tests/pty_check.py target/release/sl-neo
Standard library only; Linux and macOS.
"""

import os
import pty
import select
import signal
import struct
import sys
import termios
import fcntl
import time

BIN = sys.argv[1] if len(sys.argv) > 1 else "target/release/sl-neo"
ENTER, LEAVE, SHOW = b"\x1b[?1049h", b"\x1b[?1049l", b"\x1b[?25h"


def run(args, env=None, send=None, sig=None, at=0.6, limit=30.0):
    """Returns (exit code or -signal, seconds, output)."""
    pid, fd = pty.fork()
    if pid == 0:
        os.environ.update({"TERM": "xterm-256color", "COLORTERM": "truecolor", "LANG": "C"})
        os.environ.update(env or {})
        os.execv(BIN, [BIN] + args)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
    start, out, acted = time.time(), b"", False
    while True:
        if not acted and time.time() - start > at:
            acted = True
            if send:
                os.write(fd, send)
            if sig:
                os.kill(pid, sig)
        ready, _, _ = select.select([fd], [], [], 0.05)
        if ready:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            out += chunk
        if time.time() - start > limit:
            os.kill(pid, signal.SIGKILL)
            raise SystemExit(f"timeout: {args}")
    _, status = os.waitpid(pid, 0)
    code = os.WEXITSTATUS(status) if os.WIFEXITED(status) else -os.WTERMSIG(status)
    return code, time.time() - start, out


failures = []


def check(name, cond, detail=""):
    print(("ok   " if cond else "FAIL ") + name + (f"  ({detail})" if detail and not cond else ""))
    if not cond:
        failures.append(name)


def restored(out):
    return ENTER in out and out.rfind(LEAVE) > out.rfind(ENTER) and out.rfind(SHOW) > out.rfind(ENTER)


code, secs, out = run(["--train", "e5", "--speed", "8", "-d"])
check("a run ends by itself and restores the terminal", code == 0 and restored(out), f"code={code}")

code, secs, out = run(["--train", "yamanote", "--speed", "4"], send=b"\x03")
check("Ctrl+C is ignored without -e", code == 0 and secs > 1.5 and restored(out), f"code={code} secs={secs:.1f}")

code, secs, out = run(["--train", "yamanote", "--speed", "1", "-e"], send=b"\x03")
check("Ctrl+C stops the train with -e", code == 0 and secs < 1.5 and restored(out), f"code={code} secs={secs:.1f}")

code, secs, out = run(["--train", "yamanote", "--speed", "1", "-e"], send=b"q")
check("q stops the train with -e", code == 0 and secs < 1.5 and restored(out), f"code={code} secs={secs:.1f}")

for sig, name in [(signal.SIGTERM, "SIGTERM"), (signal.SIGHUP, "SIGHUP")]:
    code, secs, out = run(["--train", "steam", "--speed", "1"], sig=sig)
    check(f"{name} restores the terminal and exits 128+{int(sig)}", code == 128 + sig and restored(out), f"code={code}")

code, secs, out = run(["--train", "steam", "--speed", "4"], sig=signal.SIGINT)
check("SIGINT from another process is ignored without -e", code == 0 and secs > 1.0 and restored(out), f"code={code}")

code, secs, out = run(["--train", "steam", "--speed", "1", "-e"], sig=signal.SIGINT)
check("SIGINT ends the run with -e", code == 128 + signal.SIGINT and restored(out), f"code={code}")

code, secs, out = run(["--linear", "--lang", "en"])
check("the maglev leaves a closing message", code == 0 and b"That was the maglev" in out and restored(out), f"code={code}")

code, secs, out = run(["--train", "e5"], env={"TERM": "dumb"})
check("TERM=dumb prints the train once, without escapes", code == 0 and b"\x1b" not in out and b"[==]" in out, repr(out[:80]))

code, secs, out = run(["--train", "e5", "--still"])
check("--still on a terminal prints once, in color", code == 0 and ENTER not in out and b"\x1b[0;38;2;" in out, f"code={code}")

if failures:
    raise SystemExit(f"{len(failures)} check(s) failed")
