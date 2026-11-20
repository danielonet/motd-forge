"""Theme loading and rendering."""
from __future__ import annotations

import datetime as dt
import os
import tomllib
from pathlib import Path

ANSI = {"red": "31", "green": "32", "yellow": "33", "blue": "34", "magenta": "35", "cyan": "36", "white": "37"}
BOX = {"round": "╭╮╰╯─│├┤"}


def theme_dirs() -> list[Path]:
    home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    dirs = [home / "motd-forge" / "themes", Path("/usr/share/motd-forge/themes")]
    dirs.append(Path(__file__).resolve().parents[2] / "themes")  # source checkout
    extra = os.environ.get("MOTD_FORGE_THEME_DIR")
    return ([Path(extra)] if extra else []) + dirs


def list_themes() -> list[str]:
    names = {p.stem for d in theme_dirs() if d.is_dir() for p in d.glob("*.toml")}
    return sorted(names)


def load_theme(name: str) -> dict:
    for d in theme_dirs():
        p = d / f"{name}.toml"
        if p.is_file():
            with open(p, "rb") as f:
                return tomllib.load(f)
    raise FileNotFoundError(f"theme not found: {name}")


class _Ctx(dict):
    def __missing__(self, key):
        return ""


def build_context(data: dict, cache: dict, cfg: dict) -> tuple[_Ctx, list[tuple[str, str, str]]]:
    """Merge live + cached data into template strings and a list of alerts.

    Each alert is (icon, message, hint).
    """
    ctx = _Ctx({k: v for k, v in data.items() if isinstance(v, (str, int, float))})
    alerts: list[tuple[str, str, str]] = []

    for path, pct in data.get("disks", {}).items():
        if pct >= cfg["disk_alert_percent"]:
            alerts.append(("✖", f"disk {path}".ljust(15) + f"{pct:.0f}% full", f"run: du -xh {path} | sort -h | tail"))
    failed = cache.get("failed_units", [])
    for unit in failed:
        alerts.append(("✖", f"{unit}".ljust(15) + "failed", f"run: systemctl status {unit}"))
    if data.get("reboot_required"):
        alerts.append(("⚠", "reboot required", ""))

    n = cache.get("updates")
    if n is not None:
        ctx["updates"] = f"{n} pending" if n else "up to date"
        ctx["updates_short"] = f"{n} updates" if n else ""
    if "last_login" in cache:
        ctx["last_login"] = cache["last_login"]
    if cache.get("failed_ssh") is not None:
        ctx["failed_ssh"] = f"{cache['failed_ssh']} failed logins"
    if cache.get("docker") is not None:
        ctx["docker"] = f"{cache['docker']} running"
    if cache.get("weather"):
        ctx["weather"] = cache["weather"]
    ctx["failed_units"] = ", ".join(failed)
    ctx["hostname_upper"] = str(ctx["hostname"]).upper()
    ctx["alert_lines"] = "\n".join(f" {i} {m}" for i, m, _ in alerts)
    ctx["alert_hint"] = next((f" → {h}" for _, _, h in alerts if h), "")
    ctx["alert_count"] = str(len(alerts))
    return ctx, alerts


def pick_theme(cfg: dict, alerts: list, today: dt.date | None = None, override: str | None = None) -> str:
    if override:
        return override
    if alerts:
        return cfg["alert_theme"]
    today = today or dt.date.today()
    md = today.strftime("%m-%d")
    for rule in cfg.get("seasonal", []):
        a, b = rule["from"], rule["to"]
        inside = (a <= md <= b) if a <= b else (md >= a or md <= b)  # may wrap the year end
        if inside:
            return rule["theme"]
    return cfg["theme"]


def _frame(title: tuple[str, str], rows: list[tuple[str, str]], width: int) -> list[str]:
    tl, tr, bl, br, h, v, ml, mr = BOX["round"]
    inner = width - 2

    def line(text: str) -> str:
        return f"{v} {text.ljust(inner - 2)} {v}"

    def split(left: str, right: str) -> str:
        gap = max(1, inner - 2 - len(left) - len(right))
        return left + " " * gap + right

    out = [tl + h * inner + tr, line(split(*title)), ml + h * inner + mr]
    out += [line(split(label, value)) for label, value in rows]
    out.append(bl + h * inner + br)
    return out


def render(theme: dict, ctx: _Ctx) -> str:
    if "template" in theme:
        lines = []
        for tpl in theme["template"].split("\n"):
            out = tpl.format_map(ctx)
            # Drop lines that only exist for an empty value; keep intentional blanks.
            if "{" in tpl and not out.strip():
                continue
            lines.append(out)
    else:
        rows = []
        for label, tpl in theme.get("rows", []):
            value = tpl.format_map(ctx).strip()
            if value:
                rows.append((label, value))
        title = tuple(t.format_map(ctx) for t in theme.get("title", ["{hostname}", "{os}"]))
        lines = _frame(title, rows, theme.get("width", 40))  # type: ignore[arg-type]
    return "\n".join(lines)


def colorize(text: str, color: str | None, enabled: bool) -> str:
    if not enabled or not color or color not in ANSI or os.environ.get("NO_COLOR"):
        return text
    return "\n".join(f"\033[{ANSI[color]}m{ln}\033[0m" for ln in text.split("\n"))
