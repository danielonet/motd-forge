use motd_forge::cache::{Cache, FailedUnit, LastError};
use motd_forge::config::Config;
use motd_forge::modules;
use motd_forge::render::{self, Ctx};
use motd_forge::sys::Date;

const NOW: i64 = 1_800_000_000;
const H: i64 = 3600;
use std::collections::BTreeSet;

fn ctx_for(live: &modules::Live, cache: &Cache) -> (Ctx, usize) {
    let (ctx, alerts) = render::build_context(live, cache, &Config::default(), NOW);
    (ctx, alerts.len())
}

fn set(v: Vec<String>) -> BTreeSet<String> {
    v.into_iter().collect()
}

fn names(v: &[&str]) -> BTreeSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn alerts_from_demo() {
    // disk, failed unit, journal error, reboot required
    let (live, cache) = modules::demo(NOW);
    assert_eq!(ctx_for(&live, &cache).1, 4);
}

#[test]
fn no_alerts_when_healthy() {
    let (mut live, mut cache) = modules::demo(NOW);
    live.disks = vec![("/".into(), 10.0)];
    live.reboot_required = false;
    cache.failed_units.clear();
    cache.last_error = None;
    assert_eq!(ctx_for(&live, &cache).1, 0);
}

#[test]
fn alert_theme_wins() {
    let cfg = Config::default();
    assert_eq!(
        render::pick_theme(&cfg, true, Date::new(2026, 6, 1), None),
        "alert"
    );
}

#[test]
fn seasonal() {
    let cfg = Config::default();
    assert_eq!(
        render::pick_theme(&cfg, false, Date::new(2026, 12, 24), None),
        "frost"
    );
    assert_eq!(
        render::pick_theme(&cfg, false, Date::new(2026, 6, 1), None),
        "fortress"
    );
}

#[test]
fn seasonal_range_wrapping_year_end() {
    let mut cfg = Config::default();
    cfg.seasonal[0].from = "12-20".into();
    cfg.seasonal[0].to = "01-05".into();
    assert_eq!(
        render::pick_theme(&cfg, false, Date::new(2026, 1, 2), None),
        "frost"
    );
    assert_eq!(
        render::pick_theme(&cfg, false, Date::new(2026, 7, 2), None),
        "fortress"
    );
}

#[test]
fn all_themes_render() {
    let (live, cache) = modules::demo(NOW);
    let (ctx, _) = ctx_for(&live, &cache);
    let all = render::list_themes();
    assert!(all.len() >= 24, "expected the built-in themes, got {all:?}");
    for name in all {
        let out = render::render(&render::load_theme(&name).unwrap(), &ctx);
        assert!(!out.trim().is_empty(), "theme {name} rendered empty");
    }
}

#[test]
fn box_width_constant() {
    let (live, cache) = modules::demo(NOW);
    let (ctx, _) = ctx_for(&live, &cache);
    let out = render::render(&render::load_theme("fortress").unwrap(), &ctx);
    let widths: BTreeSet<usize> = out.lines().map(|l| l.chars().count()).collect();
    assert_eq!(widths.len(), 1, "{out}");
}

#[test]
fn bar() {
    assert_eq!(modules::bar(50.0, 10), "█████░░░░░ 50%");
}

#[test]
fn format_placeholders() {
    let mut ctx = Ctx::new();
    ctx.insert("a".into(), "1".into());
    assert_eq!(render::fmt("{a}-{missing}-{{lit}}", &ctx), "1--{lit}");
}

#[test]
fn groups() {
    assert_eq!(
        set(render::themes_in_group("movies")),
        names(&[
            "die-hard",
            "terminator",
            "matrix",
            "startrek",
            "fifth-element",
            "12-monkeys",
            "pulp-fiction",
            "armageddon",
            "sixth-sense",
            "unbreakable",
            "looper",
            "borg",
            "klingon",
            "vulcan",
            "khan",
            "transporter",
            "ds9"
        ])
    );
    assert_eq!(
        set(render::themes_in_group("pulp")),
        names(&[
            "pulp-fiction",
            "jack-rabbit-slims",
            "the-wolf",
            "big-kahuna",
            "royale-with-cheese"
        ])
    );
    assert_eq!(
        set(render::themes_in_group("startrek")),
        names(&[
            "startrek",
            "borg",
            "klingon",
            "vulcan",
            "khan",
            "transporter",
            "ds9"
        ])
    );
}

#[test]
fn theme_group_rotation() {
    let cfg = Config {
        theme_group: "movies".into(),
        ..Config::default()
    };
    let a = render::pick_theme(&cfg, false, Date::new(2026, 6, 1), None);
    let b = render::pick_theme(&cfg, false, Date::new(2026, 6, 2), None);
    assert!(render::themes_in_group("movies").contains(&a));
    assert_ne!(a, b);
}

#[test]
fn rejects_path_traversal_theme_names() {
    assert!(render::load_theme("../etc/passwd").is_err());
    assert!(render::load_theme("nope").is_err());
}

#[test]
fn config_overlay() {
    let mut cfg = Config::default();
    let t = motd_forge::toml::parse(
        "theme = \"neon\"\ndisks = [\"/\", \"/var\"]\n[modules]\nweather = true\n[[seasonal]]\ntheme = \"x\"\nfrom = \"01-01\"\nto = \"01-02\"\n",
    )
    .unwrap();
    cfg.apply(&t);
    assert_eq!(cfg.theme, "neon");
    assert_eq!(cfg.disks.len(), 2);
    assert!(cfg.module("weather") && cfg.module("docker"));
    assert_eq!(cfg.seasonal.len(), 1);
}

fn healthy() -> (modules::Live, Cache) {
    let (mut live, mut cache) = modules::demo(NOW);
    live.disks = vec![("/".into(), 10.0)];
    live.reboot_required = false;
    cache.failed_units.clear();
    cache.last_error = None;
    (live, cache)
}

fn error_aged(hours: i64) -> Option<LastError> {
    Some(LastError {
        ts: NOW - hours * H,
        unit: "sshd".into(),
        msg: "boom".into(),
    })
}

#[test]
fn journal_error_within_48h_alerts() {
    let (live, mut cache) = healthy();
    cache.last_error = error_aged(47);
    let (ctx, n) = ctx_for(&live, &cache);
    assert_eq!(n, 1);
    assert!(
        ctx["alert_lines"].contains("sshd: boom"),
        "{}",
        ctx["alert_lines"]
    );
    assert!(
        ctx["alert_lines"].contains("1d 23h"),
        "{}",
        ctx["alert_lines"]
    );
}

#[test]
fn journal_error_exactly_48h_still_alerts() {
    let (live, mut cache) = healthy();
    cache.last_error = error_aged(48);
    assert_eq!(ctx_for(&live, &cache).1, 1);
}

#[test]
fn journal_error_older_than_48h_is_ignored_and_theme_rotates() {
    let (live, mut cache) = healthy();
    cache.last_error = Some(LastError {
        ts: NOW - 48 * H - 1,
        unit: "sshd".into(),
        msg: "boom".into(),
    });
    let (_, n) = ctx_for(&live, &cache);
    assert_eq!(n, 0);
    // no alert, so the natural order applies: seasonal, then group, then theme
    let cfg = Config {
        theme_group: "movies".into(),
        ..Config::default()
    };
    let t = render::pick_theme(&cfg, n > 0, Date::new(2026, 6, 1), None);
    assert_ne!(t, "alert");
    assert!(render::themes_in_group("movies").contains(&t));
}

#[test]
fn stale_failed_unit_is_ignored_unknown_age_still_alerts() {
    let (live, mut cache) = healthy();
    cache.failed_units = vec![FailedUnit {
        name: "old.service".into(),
        since: Some(NOW - 72 * H),
    }];
    assert_eq!(ctx_for(&live, &cache).1, 0);
    cache.failed_units = vec![
        FailedUnit {
            name: "old.service".into(),
            since: Some(NOW - 72 * H),
        },
        FailedUnit {
            name: "new.service".into(),
            since: Some(NOW - H),
        },
        FailedUnit {
            name: "mystery.service".into(),
            since: None,
        },
    ];
    assert_eq!(ctx_for(&live, &cache).1, 2);
}

#[test]
fn state_based_alerts_do_not_age_out() {
    // a full disk or a pending reboot is a current condition, not a past event
    let (mut live, cache) = healthy();
    live.disks = vec![("/var".into(), 95.0)];
    live.reboot_required = true;
    assert_eq!(ctx_for(&live, &cache).1, 2);
}

#[test]
fn max_age_is_configurable() {
    let (live, mut cache) = healthy();
    cache.last_error = error_aged(10);
    let cfg = Config {
        alert_max_age_hours: 6.0,
        ..Config::default()
    };
    let (_, alerts) = render::build_context(&live, &cache, &cfg, NOW);
    assert!(alerts.is_empty());
}

#[test]
fn parses_journal_output() {
    let out =
        "1799990000.123456 web sshd[812]: error: kex failed\n1799980000.5 web kernel: older\n";
    let e = modules::parse_last_error(out, &[]).unwrap();
    assert_eq!(
        (e.ts, e.unit.as_str(), e.msg.as_str()),
        (1_799_990_000, "sshd", "error: kex failed")
    );
    assert!(modules::parse_last_error("-- No entries --\n", &[]).is_none());
    // ignore list skips noisy entries and falls through to the next one
    let e = modules::parse_last_error(out, &["kex failed".to_string()]).unwrap();
    assert_eq!(e.unit, "kernel");
}

#[test]
fn log_text_is_sanitised_for_the_terminal() {
    let out = "1799990000.0 web evil[1]: \u{1b}[2J\u{1b}]0;pwned\u{7}boom\tnow\n";
    let e = modules::parse_last_error(out, &[]).unwrap();
    assert!(!e.msg.chars().any(|c| c.is_control()), "{:?}", e.msg);
    let long = format!("1799990000.0 web x: {}\n", "a".repeat(500));
    assert!(
        modules::parse_last_error(&long, &[])
            .unwrap()
            .msg
            .chars()
            .count()
            <= 80
    );
}
