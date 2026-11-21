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

**motd-forge** builds the message of the day you see on SSH login or a console, as a themed,
data-driven banner. Written in Rust as one small self-contained binary: no runtime dependencies
beyond libc, no third-party crates, and the themes are built in.

**Runs on Debian 12 and 13** (any release with glibc 2.34 or newer, amd64 builds are published;
Debian 11 is too old) and on Ubuntu. How the banner reaches your login:

- **Debian:** `/etc/profile.d/motd-forge.sh` prints it at interactive login (SSH or console).
  Debian's `pam_motd` does not run `/etc/update-motd.d`, so a script there would never show up.
- **Ubuntu:** `/etc/update-motd.d/50-motd-forge` is run by `pam_motd`; the profile hook notices
  this and stays quiet so the banner is not printed twice.
- Opt out with `export MOTD_FORGE_DISABLE=1` in your shell profile, or delete the profile.d file.
  Debian's own legal text from `/etc/motd` is still printed first.

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
- **Fast login.** Slow modules are collected by a systemd timer into a cache. Login only reads the cache and `/proc`, runs under a 2 s timeout (about 2 ms in practice), and never fails loudly.
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

Files: all themes are built into the binary. Your own go in `~/.config/motd-forge/themes/` (or
`/usr/share/motd-forge/themes/` system-wide) and override a built-in theme of the same name.
The cache is `/var/cache/motd-forge/cache.txt`.

## Write a theme

A theme is a TOML file. Either a free `template` (`{name}` placeholders, `{{` and `}}` for literal braces; a line
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

One line (adds the signed apt repository, then installs; updates arrive via `apt upgrade`):

```
curl -fsSL https://danielonet.github.io/motd-forge/install.sh | sudo sh
```

The script is short, read it first: [`scripts/install.sh`](scripts/install.sh).
Setup of the repository itself is in `docs/apt-repo.md`.

Or add the repository by hand:

```
curl -fsSL https://danielonet.github.io/motd-forge/motd-forge.gpg | sudo tee /usr/share/keyrings/motd-forge.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/motd-forge.gpg] https://danielonet.github.io/motd-forge ./" | sudo tee /etc/apt/sources.list.d/motd-forge.list
sudo apt update && sudo apt install motd-forge
```

Or install the `.deb` directly (no automatic updates):

```
arch=$(dpkg --print-architecture)
curl -fsSLO "https://github.com/danielonet/motd-forge/releases/latest/download/motd-forge_${arch}.deb"
sudo apt install "./motd-forge_${arch}.deb"
```

From source:

```
cargo build --release                # target/release/motd-forge
cargo run -- preview neon --demo     # try a theme
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

dpkg-buildpackage -us -uc -b         # build the .deb (needs debhelper, cargo, rustc)
sudo apt install ../motd-forge_*.deb
```

## License

MIT, see `LICENSE`.
