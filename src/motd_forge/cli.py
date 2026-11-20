from __future__ import annotations

import argparse
import sys

from . import __version__, cache, config, modules, render


def _show(cfg: dict, theme_override: str | None, use_demo: bool, force_color: bool | None) -> str:
    if use_demo:
        data = modules.demo()
        cached = {k: data[k] for k in ("updates", "failed_units", "last_login", "failed_ssh", "docker") if k in data}
    else:
        data = modules.live(cfg)
        cached = cache.read(cfg["cache_max_age"])
    ctx, alerts = render.build_context(data, cached, cfg)
    name = render.pick_theme(cfg, alerts, override=theme_override)
    theme = render.load_theme(name)
    out = render.render(theme, ctx)
    color = cfg["color"] if force_color is None else force_color
    out = render.colorize(out, theme.get("color"), color)
    if "_error" in cfg:
        out += f"\nmotd-forge: bad config ignored ({cfg['_error']})"
    return out


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="motd-forge", description="Themed message of the day")
    ap.add_argument("--version", action="version", version=__version__)
    sub = ap.add_subparsers(dest="cmd")
    sub.add_parser("show", help="print the MOTD (used at login; reads cache only)")
    pv = sub.add_parser("preview", help="render a theme without logging out")
    pv.add_argument("theme")
    pv.add_argument("--demo", action="store_true", help="use sample data (shows alerts)")
    pv.add_argument("--no-color", action="store_true")
    sub.add_parser("refresh", help="update the slow-module cache (run by systemd timer)")
    sub.add_parser("themes", help="list available themes")
    args = ap.parse_args(argv)

    cfg = config.load()
    try:
        if args.cmd == "refresh":
            print(f"wrote {cache.write(modules.slow(cfg))}")
        elif args.cmd == "themes":
            for n in render.list_themes():
                g = render.load_theme(n).get("group")
                print(f"{n}  [{g}]" if g else n)
        elif args.cmd == "preview":
            print(_show(cfg, args.theme, args.demo, False if args.no_color else True))
        else:
            print(_show(cfg, None, False, None))
    except Exception as e:  # login must never fail loudly
        if args.cmd in (None, "show"):
            return 0
        print(f"motd-forge: {e}", file=sys.stderr)
        return 1
    return 0
