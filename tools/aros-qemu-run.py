#!/usr/bin/env python3
"""Run a freshly built AROS binary inside a QEMU-hosted AROS, for `cargo run`.

The binary is published on a small ISO which is swapped into the machine's
CD drive, then the command is typed into an open Shell window. This is the
only reliable host-to-guest path: QEMU's vvfat drive is snapshotted at start
and a guest-side write to it silently corrupts the file.

Requirements:
  * QEMU started with a monitor and a QMP socket, e.g.
        -monitor unix:/tmp/aros-monitor.sock,server,nowait
        -qmp     unix:/tmp/aros-qmp.sock,server,nowait
    and an (initially empty) IDE CD drive, e.g.
        -drive if=ide,index=2,media=cdrom
  * an AmigaShell window open and focused in the guest
  * an ISO builder: hdiutil (macOS) or xorriso/genisoimage/mkisofs (Linux)

Configuration, all optional, via the environment:
  AROS_QEMU_MONITOR   monitor socket      (default /tmp/aros-monitor.sock)
  AROS_QEMU_QMP       QMP socket          (default /tmp/aros-qmp.sock)
  AROS_CD_DEVICE      QEMU drive id       (default ide1-cd0)
  AROS_VOLUME         AROS volume name    (default AMIDEV)
  AROS_RUN_WAIT       seconds before the screenshot (default 6)
  AROS_CD_WAIT        seconds to let AROS notice the disc change (default 3)
  AROS_RUN_SHOT       where to write the screenshot (default next to the binary)
"""
import binascii
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import zlib

MONITOR = os.environ.get("AROS_QEMU_MONITOR", "/tmp/aros-monitor.sock")
QMP = os.environ.get("AROS_QEMU_QMP", "/tmp/aros-qmp.sock")
CD = os.environ.get("AROS_CD_DEVICE", "ide1-cd0")
VOLUME = os.environ.get("AROS_VOLUME", "AMIDEV")
WAIT = float(os.environ.get("AROS_RUN_WAIT", "6"))
# How long to let AROS settle after ejecting and after inserting. Too short and
# the command runs against a half-mounted volume: the Shell answers "device (or
# volume) is not mounted", or a "Please insert volume AMIDEV" requester appears
# and blocks everything until someone clicks it. Under TCG emulation the guest
# needs noticeably longer than on real hardware.
CD_WAIT = float(os.environ.get("AROS_CD_WAIT", "3"))

# Characters we can name as QEMU key codes. Anything else is refused rather
# than silently dropped, because a dropped character turns the command into a
# different, valid one.
KEYS = {
    " ": "spc", ":": "shift-semicolon", "/": "slash", ".": "dot", "-": "minus",
    "_": "shift-minus", "=": "equal", ",": "comma", ";": "semicolon",
    "+": "shift-equal", "(": "shift-9", ")": "shift-0", "?": "shift-slash",
}


def die(msg):
    print(f"aros-run: {msg}", file=sys.stderr)
    sys.exit(1)


def monitor_cmd(line):
    """Send one human-monitor command."""
    with socket.socket(socket.AF_UNIX) as s:
        s.settimeout(5)
        s.connect(MONITOR)
        time.sleep(0.2)
        try:
            s.recv(65536)          # banner
        except socket.timeout:
            pass
        s.sendall((line + "\n").encode())
        time.sleep(0.3)


class Qmp:
    def __init__(self, path):
        self.sock = socket.socket(socket.AF_UNIX)
        self.sock.settimeout(10)
        self.sock.connect(path)
        self.buf = b""
        self.read()                       # greeting
        self.cmd("qmp_capabilities")

    def read(self):
        while b"\n" not in self.buf:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise RuntimeError("QMP closed")
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return json.loads(line)

    def cmd(self, execute, **args):
        msg = {"execute": execute}
        if args:
            msg["arguments"] = args
        self.sock.sendall((json.dumps(msg) + "\n").encode())
        while True:
            r = self.read()
            if "error" in r:
                raise RuntimeError(r["error"])
            if "return" in r:
                return r["return"]

    def key(self, name):
        self.cmd("send-key", keys=[{"type": "qcode", "data": k}
                                   for k in name.split("-")])
        time.sleep(0.05)

    def type(self, text):
        for c in text:
            if c.islower() or c.isdigit():
                self.key(c)
            elif c.isupper():
                self.key("shift-" + c.lower())
            elif c in KEYS:
                self.key(KEYS[c])
            else:
                die(f"cannot type {c!r} - rename the binary to plain ASCII")


def build_iso(binary, workdir, extra=()):
    """Wrap the binary in an ISO the guest can mount.

    Two ISO paths are used in turn: QEMU keeps the inserted medium open, so
    the new image must never be the file currently in the drive.

    `extra` names further files to carry alongside it. The reason this
    exists: the guest's Shell truncates a typed command at about 255
    characters, so anything longer than that has to arrive as a file rather
    than as arguments.
    """
    stage = os.path.join(workdir, "stage")
    if os.path.isdir(stage):
        shutil.rmtree(stage)
    os.makedirs(stage)
    shutil.copy2(binary, stage)
    for path in extra:
        shutil.copy2(path, stage)

    marker = os.path.join(workdir, "current")
    previous = ""
    if os.path.exists(marker):
        with open(marker) as fh:
            previous = fh.read().strip()
    iso = os.path.join(workdir, "aros-run-b.iso" if previous.endswith("a.iso")
                       else "aros-run-a.iso")
    if os.path.exists(iso):
        os.remove(iso)
    if shutil.which("hdiutil"):
        cmd = ["hdiutil", "makehybrid", "-iso", "-joliet",
               "-default-volume-name", VOLUME, "-o", iso, stage]
    elif shutil.which("xorriso"):
        cmd = ["xorriso", "-as", "mkisofs", "-J", "-V", VOLUME, "-o", iso, stage]
    else:
        maker = shutil.which("genisoimage") or shutil.which("mkisofs")
        if not maker:
            die("no ISO builder found (need hdiutil, xorriso, genisoimage or mkisofs)")
        cmd = [maker, "-J", "-V", VOLUME, "-o", iso, stage]
    try:
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    except subprocess.CalledProcessError:
        die(f"could not build an ISO with {cmd[0]}")
    with open(marker, "w") as fh:
        fh.write(iso)
    return iso


def ppm_to_png(ppm_path, png_path):
    """QEMU's monitor writes binary PPM; convert it with the standard library."""
    data = open(ppm_path, "rb").read()
    fields, i = [], 0
    while len(fields) < 4:                      # magic, width, height, maxval
        while data[i:i + 1].isspace():
            i += 1
        j = i
        while not data[j:j + 1].isspace():
            j += 1
        fields.append(data[i:j])
        i = j
    if fields[0] != b"P6":
        die(f"unexpected screenshot format {fields[0]!r}")
    width, height = int(fields[1]), int(fields[2])
    pixels = data[i + 1:]

    stride = width * 3
    raw = bytearray()
    for row in range(height):
        raw.append(0)                           # filter: none
        raw += pixels[row * stride:(row + 1) * stride]

    def chunk(kind, payload):
        return (len(payload).to_bytes(4, "big") + kind + payload
                + binascii.crc32(kind + payload).to_bytes(4, "big"))

    header = width.to_bytes(4, "big") + height.to_bytes(4, "big") + bytes([8, 2, 0, 0, 0])
    with open(png_path, "wb") as fh:
        fh.write(b"\x89PNG\r\n\x1a\n")
        fh.write(chunk(b"IHDR", header))
        fh.write(chunk(b"IDAT", zlib.compress(bytes(raw), 6)))
        fh.write(chunk(b"IEND", b""))


def _same_screen(a, b):
    """Are the two framebuffer dumps identical? `None` if they cannot be read.

    **This does not tell you whether the program ran**, and the wording that
    uses it must not pretend otherwise. A changed screen means something
    happened — which could be a requester, the Shell echoing a command it did
    not understand, or the previous run still drawing. An unchanged one is
    usually a Shell that has lost focus, but a program that starts and draws
    nothing looks the same.

    Confirming a launch needs a signal from the program itself. There is none
    yet, so every caller of this says "unconfirmed".

    A read failure is its own answer rather than a `False` that would read as
    success — which is what the first version of this did.
    """
    try:
        with open(a, "rb") as fa, open(b, "rb") as fb:
            return fa.read() == fb.read()
    except OSError:
        return None


def main():
    if len(sys.argv) < 2:
        die("usage: aros-qemu-run.py <binary> [arguments...]")
    binary = sys.argv[1]
    name = os.path.basename(binary)
    # Anything after the binary is the program's own command line. AROS hands
    # it through the C runtime like any other system; a program that reads its
    # arguments can be told what to do without being rebuilt.
    program_args = sys.argv[2:]

    # `--carry <path>` puts a further file on the disc beside the binary and
    # rewrites the argument to name it as the guest will see it. The guest's
    # Shell truncates a typed command at about 255 characters, so a long
    # script has to travel as a file; this is how it gets there.
    carry = []
    cleaned = []
    i = 0
    while i < len(program_args):
        if program_args[i] == "--carry" and i + 1 < len(program_args):
            path = program_args[i + 1]
            if not os.path.exists(path):
                die(f"--carry: no such file: {path}")
            carry.append(path)
            cleaned.append("AMIDEV:" + os.path.basename(path))
            i += 2
            continue
        cleaned.append(program_args[i])
        i += 1
    program_args = cleaned

    for path, what in ((MONITOR, "monitor"), (QMP, "QMP")):
        if not os.path.exists(path):
            die(f"no QEMU {what} socket at {path} - is the AROS machine running?")

    # Keep the ISO on disk: QEMU holds the file open while the medium is in.
    workdir = os.path.join(tempfile.gettempdir(), "aros-run")
    os.makedirs(workdir, exist_ok=True)
    iso = build_iso(binary, workdir, carry)

    # AROS keeps serving the old disc unless it sees a real eject first.
    monitor_cmd(f"eject -f {CD}")
    time.sleep(CD_WAIT)
    monitor_cmd(f"change {CD} {iso}")
    time.sleep(CD_WAIT + 1)

    # What the screen looked like before the command was typed. "Typed the
    # command" and "the program ran" are different claims, and only the
    # second one is worth reporting — a Shell that has lost focus swallows
    # every keystroke and looks exactly like success from out here.
    shot_hint = os.environ.get("AROS_RUN_SHOT", os.path.join(workdir, f"{name}.png"))
    before = os.path.join(workdir, "before.ppm")
    monitor_cmd(f"screendump {before}")
    time.sleep(1)

    qmp = Qmp(QMP)
    command = " ".join([f"{VOLUME}:{name}", *program_args])
    qmp.type(command)
    qmp.key("ret")
    print(f"aros-run: typed {command}")

    time.sleep(WAIT)

    # Did anything happen? A screen identical to the one before the command
    # means the keystrokes went nowhere: nearly always a Shell that is not
    # the focused window. Say so rather than letting the caller assume.
    after = os.path.join(workdir, "after.ppm")
    monitor_cmd(f"screendump {after}")
    time.sleep(1)
    match = _same_screen(before, after)
    if match is None:
        print("aros-run: could not compare the screendumps; launch unconfirmed")
    elif match:
        print("aros-run: the screen did not change; launch unconfirmed")
        print("aros-run: often a Shell that is not the focused window - but not proof of it")
    else:
        print("aros-run: the screen changed; launch unconfirmed")
        print(f"aros-run: look at {shot_hint} to see what is actually there")

    shot = shot_hint
    ppm = os.path.join(workdir, "screen.ppm")
    for path in (shot, ppm):
        if os.path.exists(path):
            os.remove(path)
    monitor_cmd(f"screendump {ppm}")
    for _ in range(40):
        if os.path.exists(ppm) and os.path.getsize(ppm) > 0:
            break
        time.sleep(0.25)
    if not os.path.exists(ppm):
        die("the machine did not produce a screenshot")
    time.sleep(0.5)
    if shot.endswith(".ppm"):
        shutil.move(ppm, shot)
    else:
        ppm_to_png(ppm, shot)
        os.remove(ppm)
    print(f"aros-run: screenshot -> {shot}")


if __name__ == "__main__":
    main()
