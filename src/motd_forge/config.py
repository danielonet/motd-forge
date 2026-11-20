"""Configuration loading. Stdlib only (tomllib, Python >= 3.11)."""
from __future__ import annotations

import os
import tomllib
from pathlib import Path

DEFAULTS: dict = {
    "theme": "fortress",
    "color": True,
    "alert_theme": "alert",
    "disks": ["/"],
    "disk_alert_percent": 90,
    "cache_max_age": 3600,
    "modules": {
        "updates": True,
        "failed_units": True,
        "last_login": True,
        "failed_ssh": True,
        "docker": True,
        "quote": True,
        "weather": False,  # off by default
    },
    "weather": {"location": ""},
    # Auto-switch theme by date: {"theme": "frost", "from": "12-01", "to": "12-31"}
    "seasonal": [{"theme": "frost", "from": "12-01", "to": "12-31"}],
}


def config_paths() -> list[Path]:
    env = os.environ.get("MOTD_FORGE_CONFIG")
    if env:
        return [Path(env)]
    home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    return [Path("/etc/motd-forge/config.toml"), home / "motd-forge" / "config.toml"]


def _merge(base: dict, over: dict) -> dict:
    out = dict(base)
    for k, v in over.items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = _merge(out[k], v)
        else:
            out[k] = v
    return out


def load() -> dict:
    cfg = DEFAULTS
    for p in config_paths():
        try:
            with open(p, "rb") as f:
                cfg = _merge(cfg, tomllib.load(f))
        except (FileNotFoundError, PermissionError):
            continue
        except tomllib.TOMLDecodeError as e:
            # Never break login over a bad config.
            cfg = _merge(cfg, {"_error": f"{p}: {e}"})
    return cfg
