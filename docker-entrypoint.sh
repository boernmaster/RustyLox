#!/bin/bash
set -e

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
