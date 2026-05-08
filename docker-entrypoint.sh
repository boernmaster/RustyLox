#!/bin/bash
set -e

# ─── Ensure current UID has a passwd entry (rootless Docker / --user override)
# sudo and many tools fail with "you do not exist in the passwd database" when
# the running UID has no /etc/passwd entry.  Add a minimal entry if missing.
CUR_UID=$(id -u)
CUR_GID=$(id -g)
if ! getent passwd "$CUR_UID" >/dev/null 2>&1; then
    { echo "loxberry:x:${CUR_UID}:${CUR_GID}::/home/loxberry:/bin/bash" >> /etc/passwd \
      && echo "Registered uid=${CUR_UID} as loxberry in /etc/passwd"; } 2>/dev/null || \
      echo "Warning: could not register uid=${CUR_UID} in /etc/passwd (read-only fs)"
    echo "loxberry:x:${CUR_GID}:" >> /etc/group 2>/dev/null || true
fi

# ─── dnsmasq ─────────────────────────────────────────────────────────────────
# Permissions and sudoers were configured at image build time.
# Just start dnsmasq in the background before dropping privileges.
dnsmasq --keep-in-foreground &
echo "dnsmasq started (pid $!)"

# ─── Run daemon as loxberry ───────────────────────────────────────────────────
# Prefer runuser (requires real root), fall back to su, fall back to exec
# directly when already running as loxberry (rootless container runtimes).
if [ "$(id -u)" -eq 0 ]; then
    exec runuser -u loxberry -- /usr/local/bin/rustylox-daemon
else
    echo "Warning: entrypoint not running as root, skipping privilege drop"
    exec /usr/local/bin/rustylox-daemon
fi
