# motd-forge

Themed, data-driven message of the day for Debian/Ubuntu. Replaces static
`/etc/motd` with a script in `/etc/update-motd.d/` (run by `pam_motd` at login).

- Config: `/etc/motd-forge/config.toml`
- Themes: `/usr/share/motd-forge/themes/*.toml`, user themes in `~/.config/motd-forge/themes/`
- **Fast login:** slow modules (apt updates, failed units, last login, SSH failures,
  Docker, weather) are collected by `motd-forge-refresh.timer` into a cache; login only reads it.
- **Alert mode:** disk over threshold, failed systemd units or reboot-required switch to the `alert` theme.
- **Seasonal themes:** `[[seasonal]]` date ranges in config (`frost` in December by default).

## Usage

```
motd-forge preview fortress          # live data
motd-forge preview alert --demo      # sample data, shows alerts
motd-forge themes                    # list themes
motd-forge refresh                   # update cache (timer does this)
```

Development: `PYTHONPATH=src python3 -m motd_forge preview neon --demo`, `pytest`.
Build the package: `dpkg-buildpackage -us -uc -b`.

## Theme format

TOML. Either a free `template` (Python `str.format` placeholders such as `{hostname}`,
`{uptime}`, `{mem_bar}`, `{updates}`; lines whose placeholders are all empty are dropped),
or a boxed layout via `title = [left, right]` and `rows = [[label, value-template], ...]`.
Optional `color` (red, green, yellow, blue, magenta, cyan, white).

## Theme groups

Themes may set `group = "name"`. Set `theme_group = "movies"` in config to rotate
daily through that group (alerts and seasonal themes still take priority).
The `movies` group: `die-hard`, `terminator`, `matrix`, `startrek`, plus more Bruce Willis films: `fifth-element`, `12-monkeys`, `pulp-fiction`, `armageddon`, `sixth-sense`, `unbreakable`, `looper`.

The `pulp` group (`theme_group = "pulp"`): `pulp-fiction`, `jack-rabbit-slims`, `the-wolf`,
`big-kahuna`, `royale-with-cheese`. `group` may also be a list, e.g. `["movies", "pulp"]`.
