#!/bin/bash
set -e

# ─── dnsmasq setup ────────────────────────────────────────────────────────────
# The rustylox weather service optionally writes /etc/dnsmasq.d/rustylox-weather.conf
# to redirect weather.loxone.com DNS queries to this host so the Miniserver uses
# RustyLox as its cloud weather source (port 6066 emulator).
#
# This script runs as root so it can:
#  1. Create /etc/dnsmasq.d/ with group-write permission for the loxberry user
#  2. Configure sudo to allow the loxberry user to restart dnsmasq
#  3. Start dnsmasq
# then drops privileges and exec's the daemon as the loxberry user.

# Allow loxberry user to write dnsmasq drop-in configs
mkdir -p /etc/dnsmasq.d
chown root:loxberry /etc/dnsmasq.d
chmod 775 /etc/dnsmasq.d

# Allow loxberry to restart dnsmasq without a password
cat > /etc/sudoers.d/loxberry-dnsmasq <<'SUDOERS'
loxberry ALL=(root) NOPASSWD: /usr/sbin/service dnsmasq restart
SUDOERS
chmod 440 /etc/sudoers.d/loxberry-dnsmasq

# Configure dnsmasq to:
#  - not read /etc/resolv.conf (act as DNS-override, not full resolver)
#  - listen on all interfaces so the Miniserver can reach it
#  - include drop-in configs from /etc/dnsmasq.d/
cat > /etc/dnsmasq.conf <<'DNSMASQ_CONF'
# RustyLox dnsmasq – DNS redirect for Loxone Cloud Emulator
no-resolv
server=8.8.8.8
server=8.8.4.4
conf-dir=/etc/dnsmasq.d/,*.conf
DNSMASQ_CONF

# Start dnsmasq in the background
dnsmasq --keep-in-foreground &
DNSMASQ_PID=$!
echo "dnsmasq started (pid $DNSMASQ_PID)"

# ─── Run daemon as loxberry ───────────────────────────────────────────────────
exec su -s /bin/sh -c 'exec /usr/local/bin/rustylox-daemon' loxberry
