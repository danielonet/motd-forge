import datetime as dt

from motd_forge import config, modules, render


def _ctx(data=None):
    cfg = config.DEFAULTS
    d = data or modules.demo()
    cached = {k: d[k] for k in ("updates", "failed_units") if k in d}
    return render.build_context(d, cached, cfg)


def test_alerts_from_demo():
    _, alerts = _ctx()
    assert len(alerts) == 3


def test_no_alerts_when_healthy():
    d = modules.demo()
    d.update(disks={"/": 10.0}, reboot_required=False, failed_units=[])
    _, alerts = _ctx(d)
    assert alerts == []


def test_alert_theme_wins():
    _, alerts = _ctx()
    assert render.pick_theme(config.DEFAULTS, alerts) == "alert"


def test_seasonal():
    assert render.pick_theme(config.DEFAULTS, [], dt.date(2026, 12, 24)) == "frost"
    assert render.pick_theme(config.DEFAULTS, [], dt.date(2026, 6, 1)) == "fortress"


def test_all_themes_render():
    ctx, _ = _ctx()
    for name in render.list_themes():
        assert render.render(render.load_theme(name), ctx).strip()


def test_box_width_constant():
    ctx, _ = _ctx()
    lines = render.render(render.load_theme("fortress"), ctx).split("\n")
    assert len({len(ln) for ln in lines}) == 1


def test_bar():
    assert modules.bar(50) == "█████░░░░░ 50%"


def test_movies_group():
    assert set(render.themes_in_group("movies")) == {"die-hard", "terminator", "matrix", "startrek", "fifth-element", "12-monkeys", "pulp-fiction", "armageddon", "sixth-sense", "unbreakable", "looper",
        "borg", "klingon", "vulcan", "khan", "transporter", "ds9"}


def test_theme_group_rotation():
    cfg = {**config.DEFAULTS, "theme_group": "movies"}
    a = render.pick_theme(cfg, [], dt.date(2026, 6, 1))
    b = render.pick_theme(cfg, [], dt.date(2026, 6, 2))
    assert a in render.themes_in_group("movies") and a != b


def test_pulp_group():
    assert set(render.themes_in_group("pulp")) == {
        "pulp-fiction", "jack-rabbit-slims", "the-wolf", "big-kahuna", "royale-with-cheese"}


def test_startrek_group():
    assert set(render.themes_in_group("startrek")) == {
        "startrek", "borg", "klingon", "vulcan", "khan", "transporter", "ds9"}
