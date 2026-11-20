"""Cache for slow modules. The login script only ever reads this file."""
from __future__ import annotations

import json
import os
import time
from pathlib import Path


def cache_path() -> Path:
    env = os.environ.get("MOTD_FORGE_CACHE")
    if env:
        return Path(env)
    if os.geteuid() == 0 or Path("/var/cache/motd-forge").is_dir():
        return Path("/var/cache/motd-forge/cache.json")
    return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "motd-forge" / "cache.json"


def read(max_age: int = 3600) -> dict:
    """Return cached data, or {} if missing/stale/corrupt."""
    try:
        p = cache_path()
        if time.time() - p.stat().st_mtime > max_age:
            return {}
        return json.loads(p.read_text())
    except (OSError, ValueError):
        return {}


def write(data: dict) -> Path:
    p = cache_path()
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(data))
    os.replace(tmp, p)
    p.chmod(0o644)
    return p
