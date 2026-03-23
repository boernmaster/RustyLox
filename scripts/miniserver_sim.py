#!/usr/bin/env python3
"""
Miniserver Simulator
====================
Simulates a Loxone Miniserver sending data to a running RustyLox instance.

Tests all three inbound paths:
  1. Virtual HTTP Output  -> GET   http://<host>:8080/dev/sps/io/<name>/<value>
  2. MQTT Gateway UDP     -> UDP   <host>:11884  (JSON / simple / space / prefix format)
  3. Miniserver UDP recv  -> UDP   <host>:8090   (prefix format)
  4. PHP plugin calls     -> GET   http://<host>:<http-port>/plugins/...
                                   (Vitoconnect, sonos4lox)

Usage:
  python3 scripts/miniserver_sim.py [--host HOST] [--http-port PORT]
  python3 scripts/miniserver_sim.py --host 10.0.0.7
  python3 scripts/miniserver_sim.py --suite kaltenegger
  python3 scripts/miniserver_sim.py --suite kaltenegger --only php
"""

import argparse
import json
import socket
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Optional

# ── ANSI colours ──────────────────────────────────────────────────────────────

RESET  = "\033[0m"
BOLD   = "\033[1m"
GREEN  = "\033[32m"
YELLOW = "\033[33m"
CYAN   = "\033[36m"
RED    = "\033[31m"
DIM    = "\033[2m"

def ok(msg: str)   -> str: return f"{GREEN}  OK{RESET}  {msg}"
def err(msg: str)  -> str: return f"{RED} ERR{RESET}  {msg}"
def sent(msg: str) -> str: return f"{CYAN}SENT{RESET}  {msg}"
def hdr(msg: str)  -> str: return f"\n{BOLD}{YELLOW}{'─'*60}{RESET}\n{BOLD}{YELLOW}  {msg}{RESET}\n{BOLD}{YELLOW}{'─'*60}{RESET}"

# ── Helpers ───────────────────────────────────────────────────────────────────

def http_get(url: str, timeout: float = 3.0,
             basic_auth: tuple[str, str] | None = None,
             follow_redirects: bool = True) -> tuple[int, str]:
    """Send an HTTP GET and return (status_code, body)."""
    import base64

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, *_args, **_kwargs):
            return None

    try:
        req = urllib.request.Request(url)
        if basic_auth:
            creds = base64.b64encode(f"{basic_auth[0]}:{basic_auth[1]}".encode()).decode()
            req.add_header("Authorization", f"Basic {creds}")
        if follow_redirects:
            opener = urllib.request.build_opener()
        else:
            opener = urllib.request.build_opener(NoRedirect)
        with opener.open(req, timeout=timeout) as resp:
            return resp.status, resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode("utf-8", errors="replace")
    except Exception as e:
        return 0, str(e)


def udp_send(host: str, port: int, payload: str | bytes) -> None:
    """Send a single UDP datagram."""
    data = payload.encode() if isinstance(payload, str) else payload
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
        s.sendto(data, (host, port))


# ── Test cases ────────────────────────────────────────────────────────────────

@dataclass
class HttpTest:
    description: str
    path: str          # e.g. /dev/sps/io/Sensor/42
    expected_code: int = 200
    basic_auth: tuple[str, str] | None = None   # (user, password)
    follow_redirects: bool = True

@dataclass
class UdpTest:
    description: str
    port: int
    payload: str


# Generic sanity tests (always run)
HTTP_TESTS: list[HttpTest] = [
    HttpTest("pulse input (no value)",             "/dev/sps/io/PulseInput"),
    HttpTest("analog value",                       "/dev/sps/io/TestSensor/42.5"),
    HttpTest("digital on",                         "/dev/sps/io/Light_Kitchen/1"),
    HttpTest("digital off",                        "/dev/sps/io/Light_Kitchen/0"),
    HttpTest("value via query string",             "/dev/sps/io/Sensor?value=99.5"),
    HttpTest("url-encoded value",                  "/dev/sps/io/Status_Text/Hello%20World"),
]

UDP_11884_TESTS: list[UdpTest] = [
    UdpTest("JSON format",
            11884, '{"topic":"home/temperature","value":"23.5"}'),
    UdpTest("simple topic=value",
            11884, "home/humidity=65"),
    UdpTest("Miniserver prefix: single pair",
            11884, "MQTT: sensor_value=42"),
    UdpTest("Miniserver prefix: multi-pair",
            11884, "WeatherStation: Temp=23.5 Humidity=65 Wind=12"),
    UdpTest("bare pulse (no = sign)",
            11884, "TRIGGER"),
    UdpTest("reconnect signal",
            11884, "reconnect"),
]

UDP_8090_TESTS: list[UdpTest] = [
    UdpTest("single pair",          8090, "SensorData: Temperature=21.0"),
    UdpTest("multi-pair",           8090, "Weather: Temp=18.5 Humidity=72 Wind=8"),
    UdpTest("bare pulse",           8090, "PULSE"),
]

# Kaltenegger-specific tests (--suite kaltenegger)
HTTP_KALTENEGGER: list[HttpTest] = [
    HttpTest("Heizung Betriebsart=2 (DHW+heating)",
             "/dev/sps/io/loxberry_Heizung_Einstellung_Betriebsart/2"),
    HttpTest("Raumtemperatur Soll=21.5",
             "/dev/sps/io/loxberry_Heizung_Einstellung_Raumtemperatur/21.5"),
    HttpTest("Lueftung Betriebsart=3 (Auto)",
             "/dev/sps/io/loxberry_Lueftung_Einstellung_Betriebsart_Lueftung/3"),
    HttpTest("Sauna Temperatur=80",
             "/dev/sps/io/loxberry_Sauna_Temperatur/80"),
    HttpTest("Shelly Bad heater on",
             "/dev/sps/io/shellies_shellyplug-s-80646F81A6B8_relay_0_command/on"),
    HttpTest("Chicken door POWER1 ON",
             "/dev/sps/io/cmnd_chicken_door_POWER1/ON"),
    HttpTest("Anwesenheit FensterEG=1",
             "/dev/sps/io/loxberry_Anwesenheit_FensterEG/1"),
]

UDP_11884_KALTENEGGER: list[UdpTest] = [
    # --- Real format from Kaltenegger_MQTT.Loxone "To MQTT" VirtualOut ---
    # The Miniserver sends "TOPIC VALUE" (space-separated), NOT "TOPIC=value"
    UdpTest("Heizung Betriebsart=2 (space-sep, real MS format)",
            11884, "loxberry/Heizung/Einstellung_Betriebsart 2"),
    UdpTest("Heizung Betriebsart=3 (space-sep)",
            11884, "loxberry/Heizung/Einstellung_Betriebsart 3"),
    UdpTest("Lueftung Betriebsart=3 Auto (space-sep)",
            11884, "loxberry/Lueftung/Einstellung_Betriebsart_Lueftung 3"),
    UdpTest("Heizung Raumtemperatur Soll=21.5 (space-sep)",
            11884, "loxberry/Heizung/Einstellung_Raumtemperatur 21.5"),
    UdpTest("Heizung Warmwasser An (space-sep)",
            11884, "loxberry/Heizung/Warmwasser_An 55"),
    UdpTest("Sauna Temperatur=80 (space-sep)",
            11884, "loxberry/Sauna/Temperatur 80"),
    UdpTest("Shelly Bad relay on (space-sep)",
            11884, "shellies/shellyplug-s-80646F81A6B8/relay/0/command on"),
    UdpTest("Shelly Bad relay off (space-sep)",
            11884, "shellies/shellyplug-s-80646F81A6B8/relay/0/command off"),
    UdpTest("Chicken door open (space-sep)",
            11884, "cmnd/chicken_door/POWER1 ON"),
    UdpTest("Anwesenheit FensterEG (space-sep)",
            11884, "loxberry/Anwesenheit/FensterEG 1"),
    UdpTest("Wetter Wind (space-sep)",
            11884, "loxberry/Wetter/Wind_Aktuell 8.5"),
    UdpTest("reconnect (bare pulse)",
            11884, "reconnect"),
    # --- Also test =format for completeness ---
    UdpTest("Heizung Betriebsart=2 (= format)",
            11884, "loxberry/Heizung/Einstellung_Betriebsart=2"),
    UdpTest("Lueftung Betriebsart=3 (= format)",
            11884, "loxberry/Lueftung/Einstellung_Betriebsart_Lueftung=3"),
    UdpTest("Weather Wind_Aktuell=8.5",
            11884, "loxberry/Wetter/Wind_Aktuell=8.5"),
    UdpTest("Sauna status (prefix multi)",
            11884, "Sauna: Temperatur=80.1 SollTemperatur=85 Status=1"),
    UdpTest("Shelly relay state",
            11884, "shellies/shellyplug-s-80646F81A6B8/relay/0=1"),
    UdpTest("Anwesenheit FensterEG",
            11884, "loxberry/Anwesenheit/FensterEG=1"),
    UdpTest("reconnect MQTT",
            11884, "reconnect"),
]

UDP_8090_KALTENEGGER: list[UdpTest] = [
    UdpTest("Heizung multi-value",
            8090, "Heizung: Betriebsart=2 Raumtemperatur=21.5"),
    UdpTest("Wetter multi-value",
            8090, "Wetter: Wind=8.5 Niederschlag=0"),
    UdpTest("PV multi-value",
            8090, "PV: Leistung=4200 SOC=87 Netz=-1200"),
]

# PHP plugin HTTP tests — from Kaltenegger_MQTT.Loxone VirtualOut commands
# The Miniserver calls these URLs on the RustyLox HTTP port.
# The web-ui middleware accepts lbx_ API keys as the Basic Auth password.
# expected_code=404 means "not yet implemented in RustyLox — plugin missing"
_VITOCONNECT_AUTH = ("admin", "lbx_hE9dI3PG4zbTBcz8edoxfTWFFyzkOQl1q8yJBLFQ")

HTTP_PHP_PLUGINS: list[HttpTest] = [
    # Vitoconnect — plugin is installed; token auth passes (no 303 redirect).
    # PHP script runs but exits with error → 500 (internal PHP failure, not auth).
    # 200 would mean the plugin is fully functional.
    HttpTest("Vitoconnect: oneTimeCharge start",
             "/admin/plugins/Vitoconnect/vitoconnect.php?action=setvalue&option=heating.dhw.oneTimeCharge&value=start",
             expected_code=500,
             basic_auth=_VITOCONNECT_AUTH,
             follow_redirects=False),
    HttpTest("Vitoconnect: oneTimeCharge stop",
             "/admin/plugins/Vitoconnect/vitoconnect.php?action=setvalue&option=heating.dhw.oneTimeCharge&value=stop",
             expected_code=500,
             basic_auth=_VITOCONNECT_AUTH,
             follow_redirects=False),
    # sonos4lox — Wohnzimmer
    HttpTest("Sonos WZ: stop",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=stop",
             expected_code=404),
    HttpTest("Sonos WZ: volume=20",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=volume&volume=20",
             expected_code=404),
    HttpTest("Sonos WZ: radio FM4",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=radioplaylist&playlist=ORF+Radio+FM4&volume=20",
             expected_code=404),
    HttpTest("Sonos WZ: bass=5",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=setbass&bass=5",
             expected_code=404),
    HttpTest("Sonos WZ: treble=5",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=settreble&treble=5",
             expected_code=404),
    HttpTest("Sonos WZ: TTS alarm scharf",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=sendmessage&text=Achtung!+Alarmanlage+ist+scharf&volume=50",
             expected_code=404),
    HttpTest("Sonos WZ: getsonosinfo",
             "/plugins/sonos4lox/index.php?zone=wohnzimmer&action=getsonosinfo",
             expected_code=404),
    # sonos4lox — Kinderzimmer
    HttpTest("Sonos KiZi: stop",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&action=stop",
             expected_code=404),
    HttpTest("Sonos KiZi: volume=20",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&action=volume&volume=20",
             expected_code=404),
    HttpTest("Sonos KiZi: radio FM4",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&action=radioplaylist&playlist=Radio+FM4+92.4&volume=20",
             expected_code=404),
    HttpTest("Sonos KiZi: messageid=8 (Hexenlachen)",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&action=sendmessage&messageid=8&volume=80",
             expected_code=404),
    HttpTest("Sonos KiZi: messageid=99 (Weihnachtsglocke)",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&action=sendmessage&messageid=99&volume=100",
             expected_code=404),
    HttpTest("Sonos KiZi: wecker (messageid=5)",
             "/plugins/sonos4lox/index.php?zone=kinderzimmer&playgong=yes&action=sendmessage&messageid=5&volume=20",
             expected_code=404),
]

# ── Runner ────────────────────────────────────────────────────────────────────

def run_http_tests(host: str, port: int, tests: list[HttpTest]) -> tuple[int, int]:
    passed = failed = 0
    for t in tests:
        url = f"http://{host}:{port}{t.path}"
        status, body = http_get(url, basic_auth=t.basic_auth, follow_redirects=t.follow_redirects)
        body_preview = body.strip().replace("\n", " ")[:80]
        if status == t.expected_code:
            print(ok(f"{t.description}"))
            print(f"       {DIM}GET :{port}{t.path[:60]}{RESET}")
            print(f"       {DIM}{status} {body_preview}{RESET}")
            passed += 1
        else:
            print(err(f"{t.description}"))
            print(f"       {DIM}GET :{port}{t.path[:60]}{RESET}")
            print(f"       {DIM}expected {t.expected_code}, got {status}: {body_preview}{RESET}")
            failed += 1
        time.sleep(0.05)
    return passed, failed


def run_udp_tests(host: str, tests: list[UdpTest]) -> tuple[int, int]:
    """UDP is fire-and-forget — we just report what was sent."""
    passed = failed = 0
    for t in tests:
        try:
            udp_send(host, t.port, t.payload)
            print(sent(f"[:{t.port}] {t.description}"))
            print(f"       {DIM}{repr(t.payload)}{RESET}")
            passed += 1
        except Exception as e:
            print(err(f"[:{t.port}] {t.description}: {e}"))
            failed += 1
        time.sleep(0.05)
    return passed, failed


def run_suite(host: str, http_port: int, suite: str) -> None:
    total_pass = total_fail = 0

    # ── HTTP ──────────────────────────────────────────────────────────────────
    if suite == "kaltenegger":
        http_tests = HTTP_KALTENEGGER
        udp_11884  = UDP_11884_KALTENEGGER
        udp_8090   = UDP_8090_KALTENEGGER
        label      = "Kaltenegger Config"
    else:
        http_tests = HTTP_TESTS
        udp_11884  = UDP_11884_TESTS
        udp_8090   = UDP_8090_TESTS
        label      = "Generic"

    print(hdr(f"{label} Suite  →  {host}"))

    print(f"\n{BOLD}Virtual HTTP Output  (port {http_port}){RESET}")
    p, f = run_http_tests(host, http_port, http_tests)
    total_pass += p; total_fail += f

    print(f"\n{BOLD}MQTT Gateway UDP  (port 11884){RESET}")
    p, f = run_udp_tests(host, udp_11884)
    total_pass += p; total_fail += f

    print(f"\n{BOLD}Miniserver UDP Receiver  (port 8090){RESET}")
    p, f = run_udp_tests(host, udp_8090)
    total_pass += p; total_fail += f

    # ── Summary ───────────────────────────────────────────────────────────────
    colour = GREEN if total_fail == 0 else RED
    print(f"\n{BOLD}{colour}{'─'*60}{RESET}")
    print(f"{BOLD}{colour}  Results: {total_pass} sent/OK  |  {total_fail} failed{RESET}")
    print(f"{BOLD}{colour}{'─'*60}{RESET}\n")

    if total_fail > 0:
        print(f"{YELLOW}  Note: UDP failures are likely a network/firewall issue.{RESET}")
        print(f"{YELLOW}  UDP packets are sent but not confirmed by the app.{RESET}")
        print(f"{YELLOW}  Check RustyLox logs:  RUST_LOG=debug cargo run{RESET}\n")


# ── Main ──────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Simulate a Loxone Miniserver sending data to RustyLox.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Test against local dev instance
  python3 scripts/miniserver_sim.py

  # Test against Raspberry Pi / Docker host
  python3 scripts/miniserver_sim.py --host 10.0.0.7

  # Run Kaltenegger-specific topics
  python3 scripts/miniserver_sim.py --suite kaltenegger

  # Check only HTTP endpoint
  python3 scripts/miniserver_sim.py --only http
        """,
    )
    parser.add_argument("--host",      default="localhost", help="RustyLox host (default: localhost)")
    parser.add_argument("--http-port", default=8080, type=int, help="HTTP port (default: 8080)")
    parser.add_argument("--suite",     choices=["generic", "kaltenegger"], default="generic",
                        help="Test suite to run (default: generic)")
    parser.add_argument("--only",      choices=["http", "udp11884", "udp8090", "php"],
                        help="Run only one transport/suite")
    parser.add_argument("--delay",     default=0.05, type=float,
                        help="Delay between requests in seconds (default: 0.05)")

    args = parser.parse_args()

    suite = args.suite
    if suite == "kaltenegger":
        http_tests = HTTP_KALTENEGGER
        udp_11884  = UDP_11884_KALTENEGGER
        udp_8090   = UDP_8090_KALTENEGGER
        php_tests  = HTTP_PHP_PLUGINS
        label      = "Kaltenegger Config"
    else:
        http_tests = HTTP_TESTS
        udp_11884  = UDP_11884_TESTS
        udp_8090   = UDP_8090_TESTS
        php_tests  = []
        label      = "Generic"

    print(hdr(f"{label} Suite  ->  {args.host}"))

    total_pass = total_fail = 0

    if not args.only or args.only == "http":
        print(f"\n{BOLD}Virtual HTTP Output  (port {args.http_port}){RESET}")
        p, f = run_http_tests(args.host, args.http_port, http_tests)
        total_pass += p; total_fail += f

    if not args.only or args.only == "udp11884":
        print(f"\n{BOLD}MQTT Gateway UDP  (port 11884){RESET}")
        p, f = run_udp_tests(args.host, udp_11884)
        total_pass += p; total_fail += f

    if not args.only or args.only == "udp8090":
        print(f"\n{BOLD}Miniserver UDP Receiver  (port 8090){RESET}")
        p, f = run_udp_tests(args.host, udp_8090)
        total_pass += p; total_fail += f

    if php_tests and (not args.only or args.only == "php"):
        print(f"\n{BOLD}PHP Plugin HTTP Calls  (Vitoconnect + sonos4lox){RESET}")
        print(f"  {DIM}expected_code=404 means plugin not yet installed in RustyLox{RESET}")
        p, f = run_http_tests(args.host, args.http_port, php_tests)
        total_pass += p; total_fail += f

    colour = GREEN if total_fail == 0 else RED
    print(f"\n{BOLD}{colour}{'─'*60}{RESET}")
    print(f"{BOLD}{colour}  Results: {total_pass} sent/OK  |  {total_fail} failed{RESET}")
    print(f"{BOLD}{colour}{'─'*60}{RESET}\n")

    if total_fail > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
