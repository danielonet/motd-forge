use motd_forge::config::Config;
use motd_forge::render::{self, Ctx};
use motd_forge::sys::Date;
use motd_forge::{cache::Cache, modules};
use std::collections::BTreeSet;

fn ctx_for(live: &modules::Live, cache: &Cache) -> (Ctx, usize) {
    let (ctx, alerts) = render::build_context(live, cache, &Config::default());
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
    let (live, cache) = modules::demo();
    assert_eq!(ctx_for(&live, &cache).1, 3);
}

#[test]
fn no_alerts_when_healthy() {
    let (mut live, mut cache) = modules::demo();
    live.disks = vec![("/".into(), 10.0)];
    live.reboot_required = false;
    cache.failed_units.clear();
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
    let (live, cache) = modules::demo();
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
    let (live, cache) = modules::demo();
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
