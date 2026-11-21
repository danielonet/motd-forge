use crate::toml::{self, Table, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Seasonal {
    pub theme: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub theme: String,
    pub theme_group: String,
    pub color: bool,
    pub alert_theme: String,
    pub disks: Vec<String>,
    pub disk_alert_percent: f64,
    /// Journal errors and failed units older than this many hours no longer raise an alert.
    pub alert_max_age_hours: f64,
    /// Journal entries containing any of these substrings are not treated as errors.
    pub log_error_ignore: Vec<String>,
    pub cache_max_age: u64,
    pub modules: BTreeMap<String, bool>,
    pub weather_location: String,
    pub seasonal: Vec<Seasonal>,
    /// Set when a config file could not be parsed; shown once, never fatal.
    pub error: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let modules = [
            ("updates", true),
            ("failed_units", true),
            ("log_errors", true),
            ("last_login", true),
            ("failed_ssh", true),
            ("docker", true),
            ("quote", true),
            ("weather", false), // off by default
        ];
        Config {
            theme: "fortress".into(),
            theme_group: String::new(),
            color: true,
            alert_theme: "alert".into(),
            disks: vec!["/".into()],
            disk_alert_percent: 90.0,
            alert_max_age_hours: 48.0,
            log_error_ignore: Vec::new(),
            cache_max_age: 3600,
            modules: modules.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            weather_location: String::new(),
            seasonal: vec![Seasonal {
                theme: "frost".into(),
                from: "12-01".into(),
                to: "12-31".into(),
            }],
            error: None,
        }
    }
}

impl Config {
    pub fn module(&self, name: &str) -> bool {
        self.modules.get(name).copied().unwrap_or(false)
    }

    /// Overlay values from a parsed config file.
    pub fn apply(&mut self, t: &Table) {
        let s = |k: &str| t.get(k).and_then(Value::as_str).map(String::from);
        if let Some(v) = s("theme") {
            self.theme = v;
        }
        if let Some(v) = s("theme_group") {
            self.theme_group = v;
        }
        if let Some(v) = s("alert_theme") {
            self.alert_theme = v;
        }
        if let Some(v) = t.get("color").and_then(Value::as_bool) {
            self.color = v;
        }
        if let Some(v) = t.get("disk_alert_percent").and_then(Value::as_f64) {
            self.disk_alert_percent = v;
        }
        if let Some(v) = t.get("cache_max_age").and_then(Value::as_f64) {
            self.cache_max_age = v.max(0.0) as u64;
        }
        if let Some(v) = t.get("alert_max_age_hours").and_then(Value::as_f64) {
            self.alert_max_age_hours = v.max(0.0);
        }
        if let Some(a) = t.get("log_error_ignore").and_then(Value::as_arr) {
            self.log_error_ignore = a
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(a) = t.get("disks").and_then(Value::as_arr) {
            self.disks = a
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(m) = t.get("modules").and_then(Value::as_table) {
            for (k, v) in m {
                if let Some(b) = v.as_bool() {
                    self.modules.insert(k.clone(), b);
                }
            }
        }
        if let Some(loc) = t
            .get("weather")
            .and_then(Value::as_table)
            .and_then(|w| w.get("location"))
            .and_then(Value::as_str)
        {
            self.weather_location = loc.to_string();
        }
        if let Some(a) = t.get("seasonal").and_then(Value::as_arr) {
            self.seasonal = a
                .iter()
                .filter_map(Value::as_table)
                .filter_map(|r| {
                    let g = |k: &str| r.get(k).and_then(Value::as_str).map(String::from);
                    Some(Seasonal {
                        theme: g("theme")?,
                        from: g("from")?,
                        to: g("to")?,
                    })
                })
                .collect();
        }
    }
}

pub fn config_paths() -> Vec<PathBuf> {
    if let Some(p) = std::env::var_os("MOTD_FORGE_CONFIG") {
        return vec![PathBuf::from(p)];
    }
    vec![
        PathBuf::from("/etc/motd-forge/config.toml"),
        user_config_dir().join("config.toml"),
    ]
}

pub fn user_config_dir() -> PathBuf {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|x| !x.is_empty()) {
        return PathBuf::from(x).join("motd-forge");
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/root"));
    home.join(".config").join("motd-forge")
}

pub fn load() -> Config {
    let mut cfg = Config::default();
    for p in config_paths() {
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        match toml::parse(&text) {
            Ok(t) => cfg.apply(&t),
            // Never break login over a bad config.
            Err(e) => cfg.error = Some(format!("{}: {}", p.display(), e)),
        }
    }
    cfg
}
