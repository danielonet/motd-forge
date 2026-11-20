"""Data collectors.

`live()` is cheap (reads /proc, statvfs) and runs at login.
`slow()` shells out and may take seconds; it only runs from `motd-forge refresh`
(systemd timer) and its result is cached.
"""
from __future__ import annotations

import datetime as dt
import os
import platform
import shutil
import subprocess
from pathlib import Path

QUOTES = [
    "Premature optimization is the root of all evil. - Knuth",
    "Simplicity is prerequisite for reliability. - Dijkstra",
    "Everything fails all the time. - Vogels",
    "Make it work, make it right, make it fast. - Beck",
    "There is no cloud, just someone else's computer.",
    "Have you tried turning it off and on again?",
]


def human_duration(seconds: float) -> str:
    s = int(seconds)
    d, s = divmod(s, 86400)
    h, s = divmod(s, 3600)
    m = s // 60
    if d:
        return f"{d}d {h}h {m}m"
    return f"{h}h {m}m" if h else f"{m}m"


def bar(pct: float, width: int = 10) -> str:
    filled = max(0, min(width, round(pct / 100 * width)))
    return "█" * filled + "░" * (width - filled) + f" {pct:.0f}%"


def _os_name() -> str:
    try:
        for line in Path("/etc/os-release").read_text().splitlines():
            if line.startswith("PRETTY_NAME="):
                return line.split("=", 1)[1].strip('"')
    except OSError:
        pass
    return platform.system()


def _uptime() -> float:
    try:
        return float(Path("/proc/uptime").read_text().split()[0])
    except (OSError, ValueError):
        return 0.0


def _mem_pct() -> float | None:
    try:
        info = {}
        for line in Path("/proc/meminfo").read_text().splitlines():
            k, v = line.split(":", 1)
            info[k] = int(v.split()[0])
        return 100 * (1 - info["MemAvailable"] / info["MemTotal"])
    except (OSError, KeyError, ValueError):
        return None


def disk_usage(path: str) -> float | None:
    try:
        u = shutil.disk_usage(path)
        return 100 * u.used / u.total
    except OSError:
        return None


def live(cfg: dict) -> dict:
    d: dict = {
        "hostname": platform.node(),
        "os": _os_name(),
        "kernel": platform.release(),
        "uptime_s": _uptime(),
        "uptime": human_duration(_uptime()),
    }
    try:
        l1, l5, l15 = os.getloadavg()
        d["load"] = f"{l1:.2f} {l5:.2f} {l15:.2f}"
        d["load1"] = f"{l1:.2f}"
    except OSError:
        pass
    mem = _mem_pct()
    if mem is not None:
        d["mem_pct"] = mem
        d["mem_bar"] = bar(mem)
    disks = {}
    for p in cfg["disks"]:
        pct = disk_usage(p)
        if pct is not None:
            disks[p] = pct
    d["disks"] = disks
    if disks:
        first = next(iter(disks))
        d["disk_path"] = first
        d["disk_bar"] = bar(disks[first])
    d["reboot_required"] = Path("/run/reboot-required").exists()
    if cfg["modules"].get("quote"):
        d["quote"] = QUOTES[dt.date.today().toordinal() % len(QUOTES)]
    return d


def _run(cmd: list[str], timeout: int = 20) -> str | None:
    if not shutil.which(cmd[0]):
        return None
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return r.stdout if r.returncode == 0 else None
    except (subprocess.SubprocessError, OSError):
        return None


def slow(cfg: dict) -> dict:
    m = cfg["modules"]
    out: dict = {}
    if m.get("updates"):
        s = _run(["apt-get", "-s", "-o", "Debug::NoLocking=1", "upgrade"])
        if s is not None:
            out["updates"] = sum(1 for ln in s.splitlines() if ln.startswith("Inst "))
    if m.get("failed_units"):
        s = _run(["systemctl", "--failed", "--no-legend", "--plain"])
        if s is not None:
            out["failed_units"] = [ln.split()[0] for ln in s.splitlines() if ln.strip()]
    if m.get("last_login"):
        s = _run(["last", "-n", "2", "-F", "-w"])
        if s:
            lines = [ln for ln in s.splitlines() if ln and not ln.startswith(("wtmp", "reboot"))]
            if len(lines) > 1:  # [0] is the current session
                parts = lines[1].split()
                host = parts[2] if len(parts) > 2 else "?"
                out["last_login"] = f"from {host}"
    if m.get("failed_ssh"):
        s = _run(["journalctl", "-u", "ssh", "--since", "-24h", "--no-pager", "-q"])
        if s is not None:
            out["failed_ssh"] = sum(1 for ln in s.splitlines() if "Failed password" in ln or "Invalid user" in ln)
    if m.get("docker"):
        s = _run(["docker", "ps", "-q"], timeout=10)
        if s is not None:
            out["docker"] = len(s.split())
    if m.get("weather") and cfg["weather"].get("location"):
        loc = cfg["weather"]["location"].replace(" ", "+")
        s = _run(["curl", "-fsS", "--max-time", "8", f"https://wttr.in/{loc}?format=3"], timeout=10)
        if s:
            out["weather"] = s.strip()
    return out


def demo() -> dict:
    """Fake data for `preview --demo`."""
    return {
        "hostname": "web-prod-01", "os": "Debian 13", "kernel": "6.12.0",
        "uptime": "14d 3h 22m", "uptime_s": 14 * 86400, "load": "0.42 0.38 0.35", "load1": "0.42",
        "mem_pct": 61.0, "mem_bar": bar(61), "disks": {"/": 32.0, "/var": 94.0},
        "disk_path": "/", "disk_bar": bar(32), "reboot_required": True,
        "updates": 3, "failed_units": ["nginx.service"], "last_login": "from 10.0.0.7",
        "failed_ssh": 0, "docker": 4, "quote": QUOTES[0],
    }
