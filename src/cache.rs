//! Cache for slow modules. The login script only ever reads this file.
//! Format: one `key=value` per line (values are single-line).
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// A systemd unit in the failed state. `since` is when it failed (unix seconds), if known.
#[derive(Clone, Debug, PartialEq)]
pub struct FailedUnit {
    pub name: String,
    pub since: Option<i64>,
}

/// The most recent error-priority entry in the journal.
#[derive(Clone, Debug, PartialEq)]
pub struct LastError {
    pub ts: i64,
    pub unit: String,
    pub msg: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cache {
    pub updates: Option<u32>,
    pub failed_units: Vec<FailedUnit>,
    pub last_error: Option<LastError>,
    pub last_login: Option<String>,
    pub failed_ssh: Option<u32>,
    pub docker: Option<u32>,
    pub weather: Option<String>,
}

pub fn cache_path() -> PathBuf {
    if let Some(p) = std::env::var_os("MOTD_FORGE_CACHE") {
        return PathBuf::from(p);
    }
    if crate::sys::is_root() || PathBuf::from("/var/cache/motd-forge").is_dir() {
        return PathBuf::from("/var/cache/motd-forge/cache.txt");
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|x| !x.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/root"))
                .join(".cache")
        });
    base.join("motd-forge").join("cache.txt")
}

impl Cache {
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        let mut put = |k: &str, v: String| {
            out.push_str(&format!("{}={}\n", k, v.replace('\n', " ")));
        };
        if let Some(n) = self.updates {
            put("updates", n.to_string());
        }
        for u in &self.failed_units {
            put(
                "failed_unit",
                format!("{}\t{}", u.since.unwrap_or(0), u.name),
            );
        }
        if let Some(e) = &self.last_error {
            put("last_error", format!("{}\t{}\t{}", e.ts, e.unit, e.msg));
        }
        if let Some(v) = &self.last_login {
            put("last_login", v.clone());
        }
        if let Some(n) = self.failed_ssh {
            put("failed_ssh", n.to_string());
        }
        if let Some(n) = self.docker {
            put("docker", n.to_string());
        }
        if let Some(v) = &self.weather {
            put("weather", v.clone());
        }
        out
    }

    pub fn parse(text: &str) -> Cache {
        let mut c = Cache::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k {
                "updates" => c.updates = v.parse().ok(),
                // Older cache files: names only, failure time unknown.
                "failed_units" => {
                    c.failed_units
                        .extend(v.split(',').filter(|s| !s.is_empty()).map(|n| FailedUnit {
                            name: n.to_string(),
                            since: None,
                        }))
                }
                "failed_unit" => {
                    if let Some((ts, name)) = v.split_once('\t') {
                        c.failed_units.push(FailedUnit {
                            name: name.to_string(),
                            since: ts.parse::<i64>().ok().filter(|t| *t > 0),
                        });
                    }
                }
                "last_error" => {
                    let mut it = v.splitn(3, '\t');
                    if let (Some(ts), Some(unit), Some(msg)) = (it.next(), it.next(), it.next()) {
                        if let Ok(ts) = ts.parse() {
                            c.last_error = Some(LastError {
                                ts,
                                unit: unit.to_string(),
                                msg: msg.to_string(),
                            });
                        }
                    }
                }
                "last_login" => c.last_login = Some(v.to_string()),
                "failed_ssh" => c.failed_ssh = v.parse().ok(),
                "docker" => c.docker = v.parse().ok(),
                "weather" => c.weather = Some(v.to_string()),
                _ => {}
            }
        }
        c
    }
}

/// Cached data, or empty if missing, stale or unreadable.
pub fn read(max_age_secs: u64) -> Cache {
    let p = cache_path();
    let fresh = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| {
            SystemTime::now()
                .duration_since(t)
                .ok()
                .or(Some(Duration::ZERO))
        })
        .is_some_and(|age| age <= Duration::from_secs(max_age_secs));
    if !fresh {
        return Cache::default();
    }
    std::fs::read_to_string(&p)
        .map(|t| Cache::parse(&t))
        .unwrap_or_default()
}

pub fn write(c: &Cache) -> std::io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let p = cache_path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, c.serialize())?;
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644))?;
    std::fs::rename(&tmp, &p)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let c = Cache {
            updates: Some(3),
            failed_units: vec![
                FailedUnit {
                    name: "a.service".into(),
                    since: Some(1_700_000_000),
                },
                FailedUnit {
                    name: "b.service".into(),
                    since: None,
                },
            ],
            last_error: Some(LastError {
                ts: 1_700_000_100,
                unit: "sshd".into(),
                msg: "error: something broke".into(),
            }),
            last_login: Some("from 10.0.0.7".into()),
            failed_ssh: Some(0),
            docker: Some(4),
            weather: None,
        };
        assert_eq!(Cache::parse(&c.serialize()), c);
    }

    #[test]
    fn reads_legacy_failed_units_line() {
        let c = Cache::parse("failed_units=a.service,b.service\n");
        assert_eq!(c.failed_units.len(), 2);
        assert_eq!(c.failed_units[0].since, None);
    }
}
