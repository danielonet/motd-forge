use crate::config::Config;
use crate::{cache, config, modules, render, sys};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "motd-forge: themed message of the day

USAGE:
    motd-forge [show]                    print the MOTD (used at login; reads cache only)
    motd-forge preview <theme> [--demo] [--no-color]
                                         render a theme without logging out
    motd-forge themes                    list available themes
    motd-forge refresh                   update the slow-module cache (run by a systemd timer)
    motd-forge --version | --help
";

fn show(
    cfg: &Config,
    theme_override: Option<&str>,
    demo: bool,
    force_color: Option<bool>,
) -> Result<String, String> {
    let (live, cached) = if demo {
        modules::demo()
    } else {
        (modules::live(cfg), cache::read(cfg.cache_max_age))
    };
    let (ctx, alerts) = render::build_context(&live, &cached, cfg);
    let name = render::pick_theme(cfg, !alerts.is_empty(), sys::today(), theme_override);
    let theme = render::load_theme(&name)?;
    let mut out = render::render(&theme, &ctx);
    out = render::colorize(
        &out,
        theme.color.as_deref(),
        force_color.unwrap_or(cfg.color),
    );
    if let Some(e) = &cfg.error {
        out.push_str(&format!("\nmotd-forge: bad config ignored ({e})"));
    }
    Ok(out)
}

pub fn run(args: Vec<String>) -> i32 {
    let cmd = args.first().map(String::as_str);
    match cmd {
        Some("--version" | "-V") => {
            println!("motd-forge {VERSION}");
            return 0;
        }
        Some("--help" | "-h" | "help") => {
            print!("{HELP}");
            return 0;
        }
        _ => {}
    }
    let cfg = config::load();
    let result: Result<(), String> = match cmd {
        None | Some("show") => match show(&cfg, None, false, None) {
            Ok(s) => {
                println!("{s}");
                Ok(())
            }
            // Login must never fail loudly.
            Err(_) => return 0,
        },
        Some("preview") => {
            let rest = &args[1..];
            let demo = rest.iter().any(|a| a == "--demo");
            let no_color = rest.iter().any(|a| a == "--no-color");
            match rest.iter().find(|a| !a.starts_with("--")) {
                None => Err("preview needs a theme name (see: motd-forge themes)".into()),
                Some(theme) => {
                    show(&cfg, Some(theme), demo, Some(!no_color)).map(|s| println!("{s}"))
                }
            }
        }
        Some("themes") => {
            for n in render::list_themes() {
                let groups = render::load_theme(&n)
                    .map(|t| t.group.join(", "))
                    .unwrap_or_default();
                if groups.is_empty() {
                    println!("{n}");
                } else {
                    println!("{n}  [{groups}]");
                }
            }
            Ok(())
        }
        Some("refresh") => cache::write(&modules::slow(&cfg))
            .map(|p| println!("wrote {}", p.display()))
            .map_err(|e| format!("cannot write cache: {e}")),
        Some(other) => Err(format!("unknown command '{other}'\n\n{HELP}")),
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("motd-forge: {e}");
            1
        }
    }
}
