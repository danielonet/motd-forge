# /etc/profile.d/motd-forge.sh: show the motd-forge banner at interactive login.
# Debian's pam_motd does not run /etc/update-motd.d, so this hook does the job there.
# On systems where PAM already ran /etc/update-motd.d/50-motd-forge a moment ago
# (Ubuntu) it stays quiet, so the banner is never printed twice.
# Opt out: remove this file, or `export MOTD_FORGE_DISABLE=1` in your profile.
if [ -z "${MOTD_FORGE_SHOWN:-}${MOTD_FORGE_DISABLE:-}" ] && [ -t 1 ] \
    && command -v motd-forge >/dev/null 2>&1; then
    case $- in
    *i*)
        _mf_now=$(date +%s 2>/dev/null || echo 0)
        _mf_last=$(stat -c %Y /run/motd-forge.stamp 2>/dev/null || echo 0)
        if [ $((_mf_now - _mf_last)) -gt 10 ]; then
            MOTD_FORGE_SHOWN=1
            export MOTD_FORGE_SHOWN
            timeout 2 motd-forge show 2>/dev/null
        fi
        unset _mf_now _mf_last
        ;;
    esac
fi
