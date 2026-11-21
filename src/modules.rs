//! Data collectors.
//!
//! `live()` is cheap (reads /proc, statvfs) and runs at login.
//! `slow()` shells out and may take seconds; it only runs from `motd-forge refresh`
//! (systemd timer) and its result is cached.
use crate::cache::{Cache, FailedUnit, LastError};
use crate::config::Config;
use crate::sys;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const QUOTES: &[&str] = &[
    "Premature optimization is the root of all evil. - Knuth",
    "Simplicity is prerequisite for reliability. - Dijkstra",
    "Everything fails all the time. - Vogels",
    "Make it work, make it right, make it fast. - Beck",
    "There is no cloud, just someone else's computer.",
    "Have you tried turning it off and on again?",
];

#[derive(Clone, Debug, Default)]
pub struct Live {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub uptime_secs: f64,
    pub load: Option<(f64, f64, f64)>,
    pub mem_pct: Option<f64>,
    /// (mount path, used percent), in configured order.
    pub disks: Vec<(String, f64)>,
    pub reboot_required: bool,
    pub quote: Option<String>,
}

pub fn human_duration(seconds: f64) -> String {
    let mut s = seconds as u64;
    let d = s / 86400;
    s %= 86400;
    let h = s / 3600;
    s %= 3600;
    let m = s / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

pub fn bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0 * width as f64).round_ties_even().max(0.0) as usize).min(width);
    format!(
        "{}{} {:.0}%",
        "█".repeat(filled),
        "░".repeat(width - filled),
        pct
    )
}

fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

fn os_name() -> String {
    if let Ok(t) = std::fs::read_to_string("/etc/os-release") {
        for line in t.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                return v.trim_matches('"').to_string();
            }
        }
    }
    "Linux".into()
}

fn mem_pct() -> Option<f64> {
    let t = std::fs::read_to_string("/proc/meminfo").ok()?;
    let get = |key: &str| -> Option<f64> {
        t.lines().find_map(|l| {
            l.strip_prefix(key)?
                .strip_prefix(':')?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
    };
    let (total, avail) = (get("MemTotal")?, get("MemAvailable")?);
    (total > 0.0).then(|| 100.0 * (1.0 - avail / total))
}

fn loadavg() -> Option<(f64, f64, f64)> {
    let t = std::fs::read_to_string("/proc/loadavg").ok()?;
    let mut it = t.split_whitespace().map(|x| x.parse::<f64>());
    Some((it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?))
}

pub fn live(cfg: &Config) -> Live {
    Live {
        hostname: read_trim("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".into()),
        os: os_name(),
        kernel: read_trim("/proc/sys/kernel/osrelease").unwrap_or_default(),
        uptime_secs: read_trim("/proc/uptime")
            .and_then(|s| s.split_whitespace().next().and_then(|x| x.parse().ok()))
            .unwrap_or(0.0),
        load: loadavg(),
        mem_pct: mem_pct(),
        disks: cfg
            .disks
            .iter()
            .filter_map(|p| sys::disk_used_percent(p).map(|pct| (p.clone(), pct)))
            .collect(),
        reboot_required: std::path::Path::new("/run/reboot-required").exists(),
        quote: cfg
            .module("quote")
            .then(|| QUOTES[(sys::today().ordinal() as usize) % QUOTES.len()].to_string()),
    }
}

fn uptime_secs() -> f64 {
    read_trim("/proc/uptime")
        .and_then(|s| s.split_whitespace().next().and_then(|x| x.parse().ok()))
        .unwrap_or(0.0)
}

/// When a unit entered its current (failed) state, as unix seconds; None if unknown.
fn unit_failed_at(name: &str, boot: i64) -> Option<i64> {
    let out = run(
        "systemctl",
        &[
            "show",
            "-p",
            "StateChangeTimestampMonotonic",
            "--value",
            name,
        ],
        Duration::from_secs(5),
    )?;
    let micros: i64 = out.trim().parse().ok()?;
    (micros > 0).then(|| boot + micros / 1_000_000)
}

/// Make untrusted log text safe to print on a terminal: control characters
/// (including ESC) become spaces, and the length is capped.
pub fn sanitize(s: &str, max_chars: usize) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() > max_chars {
        let cut: String = cleaned.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    } else {
        cleaned
    }
}

/// Newest usable entry from `journalctl -o short-unix -r` output
/// (lines look like `1763750000.123456 host unit[pid]: message`).
pub fn parse_last_error(out: &str, ignore: &[String]) -> Option<LastError> {
    for line in out.lines() {
        let mut it = line.splitn(3, ' ');
        let Some(ts) = it.next().and_then(|t| t.parse::<f64>().ok()) else {
            continue; // e.g. "-- No entries --"
        };
        let _host = it.next();
        let rest = it.next().unwrap_or("");
        if ignore
            .iter()
            .any(|p| !p.is_empty() && line.contains(p.as_str()))
        {
            continue;
        }
        let (unit, msg) = rest.split_once(": ").unwrap_or(("", rest));
        let unit = unit.split('[').next().unwrap_or(unit);
        return Some(LastError {
            ts: ts as i64,
            unit: sanitize(unit, 30),
            msg: sanitize(msg, 80),
        });
    }
    None
}

/// Run a command with a timeout; None if it is missing, fails, or times out.
fn run(cmd: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    // Drain stdout on a thread so a chatty command cannot block on a full pipe.
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break Some(st),
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let text = reader.join().ok()?;
    status.filter(|s| s.success()).map(|_| text)
}

pub fn slow(cfg: &Config) -> Cache {
    let secs = |n| Duration::from_secs(n);
    let mut c = Cache::default();
    if cfg.module("updates") {
        c.updates = run(
            "apt-get",
            &["-s", "-o", "Debug::NoLocking=1", "upgrade"],
            secs(20),
        )
        .map(|s| s.lines().filter(|l| l.starts_with("Inst ")).count() as u32);
    }
    if cfg.module("failed_units") {
        if let Some(s) = run(
            "systemctl",
            &["--failed", "--no-legend", "--plain"],
            secs(20),
        ) {
            let now = sys::now_secs();
            let boot = now - uptime_secs() as i64;
            c.failed_units = s
                .lines()
                .filter_map(|l| l.split_whitespace().next())
                .map(|name| FailedUnit {
                    name: name.to_string(),
                    since: unit_failed_at(name, boot),
                })
                .collect();
        }
    }
    if cfg.module("log_errors") {
        let mins = (cfg.alert_max_age_hours * 60.0).ceil().max(1.0) as u64;
        let since = format!("-{mins}min");
        c.last_error = run(
            "journalctl",
            &[
                "-p",
                "err",
                "--since",
                &since,
                "--no-pager",
                "-q",
                "-o",
                "short-unix",
                "-r",
                "-n",
                "200",
            ],
            secs(20),
        )
        .and_then(|s| parse_last_error(&s, &cfg.log_error_ignore));
    }
    if cfg.module("last_login") {
        c.last_login = run("last", &["-n", "2", "-F", "-w"], secs(20)).and_then(|s| {
            let lines: Vec<&str> = s
                .lines()
                .filter(|l| !l.is_empty() && !l.starts_with("wtmp") && !l.starts_with("reboot"))
                .collect();
            // lines[0] is the current session
            let host = lines.get(1)?.split_whitespace().nth(2).unwrap_or("?");
            Some(format!("from {host}"))
        });
    }
    if cfg.module("failed_ssh") {
        c.failed_ssh = run(
            "journalctl",
            &["-u", "ssh", "--since", "-24h", "--no-pager", "-q"],
            secs(20),
        )
        .map(|s| {
            s.lines()
                .filter(|l| l.contains("Failed password") || l.contains("Invalid user"))
                .count() as u32
        });
    }
    if cfg.module("docker") {
        c.docker =
            run("docker", &["ps", "-q"], secs(10)).map(|s| s.split_whitespace().count() as u32);
    }
    if cfg.module("weather") && !cfg.weather_location.is_empty() {
        let url = format!(
            "https://wttr.in/{}?format=3",
            cfg.weather_location.replace(' ', "+")
        );
        c.weather = run("curl", &["-fsS", "--max-time", "8", &url], secs(10))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    c
}

/// Fake data for `preview --demo`.
pub fn demo(now: i64) -> (Live, Cache) {
    let live = Live {
        hostname: "web-prod-01".into(),
        os: "Debian 13".into(),
        kernel: "6.12.0".into(),
        uptime_secs: (14 * 86400 + 3 * 3600 + 22 * 60) as f64,
        load: Some((0.42, 0.38, 0.35)),
        mem_pct: Some(61.0),
        disks: vec![("/".into(), 32.0), ("/var".into(), 94.0)],
        reboot_required: true,
        quote: Some(QUOTES[0].into()),
    };
    let cache = Cache {
        updates: Some(3),
        failed_units: vec![FailedUnit {
            name: "nginx.service".into(),
            since: Some(now - 5 * 3600),
        }],
        last_error: Some(LastError {
            ts: now - 3 * 3600,
            unit: "sshd".into(),
            msg: "error: maximum authentication attempts exceeded".into(),
        }),
        last_login: Some("from 10.0.0.7".into()),
        failed_ssh: Some(0),
        docker: Some(4),
        weather: None,
    };
    (live, cache)
}
