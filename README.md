```
                   _        _          __
 _ __ ___    ___  | |_   __| |        / _|  ___   _ __   __ _   ___
| '_ ` _ \  / _ \ | __| / _` | _____ | |_  / _ \ | '__| / _` | / _ \
| | | | | || (_) || |_ | (_| ||_____||  _|| (_) || |   | (_| ||  __/
|_| |_| |_| \___/  \__| \__,_|       |_|   \___/ |_|    \__, | \___|
                                                        |___/
   ╭────────────────────────────────────────────────╮
   │  forge your login banner · themes · alerts · ★ │
   ╰────────────────────────────────────────────────╯
```

**motd-forge** builds the message of the day you see on SSH login or a console. It replaces the
static `/etc/motd` with a themed, data-driven banner: a script in `/etc/update-motd.d/` that
`pam_motd` runs at login on Debian and Ubuntu. Pure Python 3.11+, standard library only.

```
╭────────────────────────────────────╮
│ web-prod-01              Debian 13 │
├────────────────────────────────────┤
│ uptime                  14d 3h 22m │
│ load                0.42 0.38 0.35 │
│ memory              ██████░░░░ 61% │
│ disk                ███░░░░░░░ 32% │
│ updates                  3 pending │
╰────────────────────────────────────╯
```

## The good stuff

- **Themes in plain TOML.** Swap the look with one config line, or write your own.
- **Fast login.** Slow modules are collected by a systemd timer into a cache. Login only reads the cache and `/proc`, runs under a 2 s timeout, and never fails loudly.
- **Alert mode.** A disk over the threshold, a failed systemd unit or a pending reboot switches the banner to the red `alert` theme, with a hint on what to run.
- **Seasonal themes.** Date ranges switch themes automatically (`frost` in December by default).
- **Theme groups.** Set `theme_group = "movies"` to rotate daily through a group.
- **Preview without logging out.** `motd-forge preview <theme>`, or `--demo` for sample data that includes alerts.
- **Module set.** Hostname and OS, uptime, load, memory and disk bars, pending apt updates, `reboot-required`, failed systemd units, last login, failed SSH attempts, Docker container count, quote of the day, and optional weather (off by default).
- **Debian packaging.** `debian/` builds a `.deb`, and GitHub Actions builds it on every push and attaches it to tagged releases.

## Themes

| Group | Themes |
|---|---|
| core | `fortress` (boxed dashboard), `minimal` (one line), `neon` (cyberpunk), `frost` (seasonal), `alert` (automatic) |
| `movies` | `die-hard`, `terminator`, `matrix`, `startrek`, `fifth-element`, `12-monkeys`, `pulp-fiction`, `armageddon`, `sixth-sense`, `unbreakable`, `looper`, plus all of `startrek` |
| `pulp` | `pulp-fiction`, `jack-rabbit-slims`, `the-wolf`, `big-kahuna`, `royale-with-cheese` |
| `startrek` | `startrek`, `borg`, `klingon`, `vulcan`, `khan`, `transporter`, `ds9` |

A few of them:

```
  ┌─┬─┬─┬─┬─┬─┐                  ▐▌  ▐▌
  ├─┼─┼─┼─┼─┼─┤  WE ARE THE BORG  ▐▌▐▌▐▌▐▌   VULCAN SCIENCE ACADEMY
  └─┴─┴─┴─┴─┴─┘  Resistance is    ▐█████▌▐▌  Live long and prosper.
                 futile.
```

List them all with `motd-forge themes`.

## Usage

```
motd-forge show                      # what login runs (cache only)
motd-forge preview fortress          # live data
motd-forge preview alert --demo      # sample data, shows alerts
motd-forge preview matrix --no-color
motd-forge themes                    # list themes and their groups
motd-forge refresh                   # update the cache (the timer does this)
```

## Configuration

`/etc/motd-forge/config.toml` (user override: `~/.config/motd-forge/config.toml`):

```toml
theme = "fortress"
# theme_group = "movies"      # rotate daily through a group
color = true
disks = ["/"]
disk_alert_percent = 90

[modules]
updates = true
failed_units = true
last_login = true
failed_ssh = true
docker = true
quote = true
weather = false

[[seasonal]]
theme = "frost"
from = "12-01"
to = "12-31"
```

Theme choice priority: alerts, then seasonal ranges, then `theme_group`, then `theme`.

Files: themes in `/usr/share/motd-forge/themes/`, your own in `~/.config/motd-forge/themes/`,
cache in `/var/cache/motd-forge/cache.json`.

## Write a theme

A theme is a TOML file. Either a free `template` (Python `str.format` placeholders; a line
whose placeholders are all empty is dropped):

```toml
name = "mine"
group = "custom"            # string or list of strings, optional
color = "green"             # red green yellow blue magenta cyan white
template = '''
 {hostname} · up {uptime} · {mem_bar}
 {updates}
'''
```

or a boxed layout with `title = ["{hostname}", "{os}"]` and `rows = [["label", "{value}"], ...]`.

Placeholders: `hostname` `hostname_upper` `os` `kernel` `uptime` `load` `load1` `mem_bar`
`disk_bar` `disk_path` `updates` `updates_short` `last_login` `failed_ssh` `docker` `weather`
`quote` `failed_units` `alert_lines` `alert_hint` `alert_count`.

## Install and develop

```
dpkg-buildpackage -us -uc -b         # build the .deb
sudo apt install ../motd-forge_*.deb

pip install -e . pytest && pytest    # develop
PYTHONPATH=src python3 -m motd_forge preview neon --demo
```

## License

MIT, see `LICENSE`.
