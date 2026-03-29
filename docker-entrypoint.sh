#!/bin/bash
set -e

# ─── dnsmasq ─────────────────────────────────────────────────────────────────
# Permissions and sudoers were configured at image build time.
# Just start dnsmasq in the background before dropping privileges.
dnsmasq --keep-in-foreground &
echo "dnsmasq started (pid $!)"

# ─── Run daemon as loxberry ───────────────────────────────────────────────────
exec su -s /bin/sh -c 'exec /usr/local/bin/rustylox-daemon' loxberry
