//! Theme loading and rendering.
use crate::cache::Cache;
use crate::config::{self, Config};
use crate::modules::{bar, human_duration, Live};
use crate::sys::Date;
use crate::toml::{self, Table, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/themes_gen.rs"));
}
use generated::EMBEDDED;

pub type Ctx = BTreeMap<String, String>;

/// (icon, message, hint)
pub type Alert = (String, String, String);

#[derive(Clone, Debug, Default)]
pub struct Theme {
    pub template: Option<String>,
    pub title: Vec<String>,
    pub rows: Vec<(String, String)>,
    pub width: usize,
    pub color: Option<String>,
    pub group: Vec<String>,
}

fn ansi(color: &str) -> Option<&'static str> {
    Some(match color {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        _ => return None,
    })
}

/// Directories searched before the built-in themes (later wins nothing; first match wins).
pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("MOTD_FORGE_THEME_DIR") {
        dirs.push(PathBuf::from(d));
    }
    dirs.push(config::user_config_dir().join("themes"));
    dirs.push(PathBuf::from("/usr/share/motd-forge/themes"));
    dirs
}

pub fn list_themes() -> Vec<String> {
    let mut names: Vec<String> = EMBEDDED.iter().map(|(n, _)| n.to_string()).collect();
    for d in theme_dirs() {
        let Ok(rd) = std::fs::read_dir(d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "toml") {
                if let Some(s) = p.file_stem() {
                    names.push(s.to_string_lossy().into_owned());
                }
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

pub fn load_theme(name: &str) -> Result<Theme, String> {
    // Reject path tricks like "../x"; theme names are plain file stems.
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return Err(format!("invalid theme name: {name}"));
    }
    for d in theme_dirs() {
        let p = d.join(format!("{name}.toml"));
        if let Ok(text) = std::fs::read_to_string(&p) {
            return parse_theme(&text).map_err(|e| format!("{}: {}", p.display(), e));
        }
    }
    match EMBEDDED.iter().find(|(n, _)| *n == name) {
        Some((_, text)) => parse_theme(text).map_err(|e| format!("built-in theme {name}: {e}")),
        None => Err(format!("theme not found: {name}")),
    }
}

pub fn parse_theme(text: &str) -> Result<Theme, String> {
    let t: Table = toml::parse(text)?;
    let s = |k: &str| t.get(k).and_then(Value::as_str).map(String::from);
    let strs = |v: Option<&Value>| -> Vec<String> {
        match v {
            Some(Value::Str(x)) => vec![x.clone()],
            Some(Value::Arr(a)) => a
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
            _ => Vec::new(),
        }
    };
    let rows = t
        .get("rows")
        .and_then(Value::as_arr)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_arr)
                .filter(|r| r.len() == 2)
                .filter_map(|r| Some((r[0].as_str()?.to_string(), r[1].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let mut title = strs(t.get("title"));
    if title.is_empty() {
        title = vec!["{hostname}".into(), "{os}".into()];
    }
    Ok(Theme {
        template: s("template"),
        title,
        rows,
        width: t
            .get("width")
            .and_then(Value::as_f64)
            .map(|w| w as usize)
            .unwrap_or(40),
        color: s("color"),
        group: strs(t.get("group")),
    })
}

pub fn themes_in_group(group: &str) -> Vec<String> {
    list_themes()
        .into_iter()
        .filter(|n| load_theme(n).is_ok_and(|t| t.group.iter().any(|g| g == group)))
        .collect()
}

fn pad(s: &str, width: usize) -> String {
    let n = s.chars().count();
    format!("{}{}", s, " ".repeat(width.saturating_sub(n)))
}

/// Merge live + cached data into template strings and a list of alerts.
/// An error with a known time only counts while it is recent; unknown times always count.
fn is_recent(since: Option<i64>, now: i64, max_secs: i64) -> bool {
    match since {
        Some(t) if t > 0 => now.saturating_sub(t) <= max_secs,
        _ => true,
    }
}

/// `now` is unix seconds. Failed units and journal errors older than
/// `alert_max_age_hours` raise no alert, so the banner falls back to the normal theme rotation.
pub fn build_context(live: &Live, cache: &Cache, cfg: &Config, now: i64) -> (Ctx, Vec<Alert>) {
    let max_secs = (cfg.alert_max_age_hours * 3600.0) as i64;
    let mut ctx = Ctx::new();
    let mut alerts: Vec<Alert> = Vec::new();
    let mut put = |k: &str, v: String| {
        ctx.insert(k.to_string(), v);
    };

    put("hostname", live.hostname.clone());
    put("hostname_upper", live.hostname.to_uppercase());
    put("os", live.os.clone());
    put("kernel", live.kernel.clone());
    put("uptime", human_duration(live.uptime_secs));
    if let Some((a, b, c)) = live.load {
        put("load", format!("{a:.2} {b:.2} {c:.2}"));
        put("load1", format!("{a:.2}"));
    }
    if let Some(m) = live.mem_pct {
        put("mem_bar", bar(m, 10));
    }
    if let Some((path, pct)) = live.disks.first() {
        put("disk_path", path.clone());
        put("disk_bar", bar(*pct, 10));
    }
    if let Some(q) = &live.quote {
        put("quote", q.clone());
    }

    for (path, pct) in &live.disks {
        if *pct >= cfg.disk_alert_percent {
            alerts.push((
                "✖".into(),
                format!("{}{:.0}% full", pad(&format!("disk {path}"), 15), pct),
                format!("run: du -xh {path} | sort -h | tail"),
            ));
        }
    }
    for u in &cache.failed_units {
        if is_recent(u.since, now, max_secs) {
            alerts.push((
                "✖".into(),
                format!("{}failed", pad(&u.name, 15)),
                format!("run: systemctl status {}", u.name),
            ));
        }
    }
    if let Some(e) = &cache.last_error {
        if is_recent(Some(e.ts), now, max_secs) {
            let what = if e.unit.is_empty() {
                e.msg.clone()
            } else {
                format!("{}: {}", e.unit, e.msg)
            };
            alerts.push((
                "✖".into(),
                format!(
                    "error {} ago: {what}",
                    human_duration((now - e.ts).max(0) as f64)
                ),
                format!(
                    "run: journalctl -p err --since -{}h",
                    cfg.alert_max_age_hours
                ),
            ));
        }
    }
    if live.reboot_required {
        alerts.push(("⚠".into(), "reboot required".into(), String::new()));
    }

    if let Some(n) = cache.updates {
        put(
            "updates",
            if n > 0 {
                format!("{n} pending")
            } else {
                "up to date".into()
            },
        );
        put(
            "updates_short",
            if n > 0 {
                format!("{n} updates")
            } else {
                String::new()
            },
        );
    }
    if let Some(v) = &cache.last_login {
        put("last_login", v.clone());
    }
    if let Some(n) = cache.failed_ssh {
        put("failed_ssh", format!("{n} failed logins"));
    }
    if let Some(n) = cache.docker {
        put("docker", format!("{n} running"));
    }
    if let Some(w) = &cache.weather {
        put("weather", w.clone());
    }
    put(
        "failed_units",
        cache
            .failed_units
            .iter()
            .map(|u| u.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    put(
        "alert_lines",
        alerts
            .iter()
            .map(|(i, m, _)| format!(" {i} {m}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    put(
        "alert_hint",
        alerts
            .iter()
            .find(|a| !a.2.is_empty())
            .map(|a| format!(" → {}", a.2))
            .unwrap_or_default(),
    );
    put("alert_count", alerts.len().to_string());
    (ctx, alerts)
}

pub fn pick_theme(
    cfg: &Config,
    has_alerts: bool,
    today: Date,
    override_name: Option<&str>,
) -> String {
    if let Some(o) = override_name {
        return o.to_string();
    }
    if has_alerts {
        return cfg.alert_theme.clone();
    }
    let md = today.md();
    for r in &cfg.seasonal {
        let inside = if r.from <= r.to {
            r.from <= md && md <= r.to
        } else {
            md >= r.from || md <= r.to
        };
        if inside {
            return r.theme.clone();
        }
    }
    if !cfg.theme_group.is_empty() {
        let members = themes_in_group(&cfg.theme_group);
        if !members.is_empty() {
            // rotate daily
            return members[today.ordinal().rem_euclid(members.len() as i64) as usize].clone();
        }
    }
    cfg.theme.clone()
}

/// `{name}` substitution; unknown names render empty, `{{` and `}}` are literal braces.
pub fn fmt(tpl: &str, ctx: &Ctx) -> String {
    let cs: Vec<char> = tpl.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        match cs[i] {
            '{' if cs.get(i + 1) == Some(&'{') => {
                out.push('{');
                i += 2;
            }
            '}' if cs.get(i + 1) == Some(&'}') => {
                out.push('}');
                i += 2;
            }
            '{' => match cs[i + 1..].iter().position(|c| *c == '}') {
                Some(end) => {
                    let name: String = cs[i + 1..i + 1 + end].iter().collect();
                    out.push_str(ctx.get(name.trim()).map(String::as_str).unwrap_or(""));
                    i += end + 2;
                }
                None => {
                    out.push('{');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn frame(title: (String, String), rows: &[(String, String)], width: usize) -> Vec<String> {
    let inner = width.saturating_sub(2);
    let line = |text: String| format!("│ {} │", pad(&text, inner.saturating_sub(2)));
    let split = |l: &str, r: &str| {
        let gap = inner
            .saturating_sub(2)
            .saturating_sub(l.chars().count() + r.chars().count())
            .max(1);
        format!("{l}{}{r}", " ".repeat(gap))
    };
    let bar = "─".repeat(inner);
    let mut out = vec![
        format!("╭{bar}╮"),
        line(split(&title.0, &title.1)),
        format!("├{bar}┤"),
    ];
    out.extend(rows.iter().map(|(l, v)| line(split(l, v))));
    out.push(format!("╰{bar}╯"));
    out
}

pub fn render(theme: &Theme, ctx: &Ctx) -> String {
    if let Some(tpl) = &theme.template {
        let mut lines = Vec::new();
        for l in tpl.split('\n') {
            let out = fmt(l, ctx);
            // Drop lines that only exist for an empty value; keep intentional blanks.
            if l.contains('{') && out.trim().is_empty() {
                continue;
            }
            lines.push(out);
        }
        lines.join("\n")
    } else {
        let rows: Vec<(String, String)> = theme
            .rows
            .iter()
            .filter_map(|(label, tpl)| {
                let v = fmt(tpl, ctx).trim().to_string();
                (!v.is_empty()).then(|| (fmt(label, ctx), v))
            })
            .collect();
        let t = |k: usize| theme.title.get(k).map(|s| fmt(s, ctx)).unwrap_or_default();
        frame((t(0), t(1)), &rows, theme.width).join("\n")
    }
}

pub fn colorize(text: &str, color: Option<&str>, enabled: bool) -> String {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    match color.and_then(ansi) {
        Some(code) if enabled && !no_color => text
            .split('\n')
            .map(|l| format!("\x1b[{code}m{l}\x1b[0m"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => text.to_string(),
    }
}
