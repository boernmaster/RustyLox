#!/bin/bash
# LoxBerry System Bash Library
# Provides system path variables and utility functions for LoxBerry plugins
#
# Compatible with original LoxBerry loxberry_system.sh
# Source this file in plugin scripts: . $LBHOMEDIR/libs/bashlib/loxberry_system.sh

# Version
LBSYSTEMVERSION="3.0.0.1"

# Determine LBHOMEDIR
if [ -z "$LBHOMEDIR" ]; then
    if [ -d "/opt/loxberry" ]; then
        LBHOMEDIR="/opt/loxberry"
    else
        # Try to detect from loxberry user home
        LBHOMEDIR=$(getent passwd loxberry 2>/dev/null | cut -d: -f6)
        if [ -z "$LBHOMEDIR" ]; then
            LBHOMEDIR="/opt/loxberry"
        fi
    fi
fi
export LBHOMEDIR

# System directories
LBSHTMLDIR="$LBHOMEDIR/webfrontend/html/system"
LBSHTMLAUTHDIR="$LBHOMEDIR/webfrontend/htmlauth/system"
LBSTEMPLATEDIR="$LBHOMEDIR/templates/system"
LBSDATADIR="$LBHOMEDIR/data/system"
LBSLOGDIR="$LBHOMEDIR/log/system"
LBSTMPFSLOGDIR="$LBHOMEDIR/log/system_tmpfs"
LBSCONFIGDIR="$LBHOMEDIR/config/system"
LBSSBINDIR="$LBHOMEDIR/sbin"
LBSBINDIR="$LBHOMEDIR/bin"

export LBSHTMLDIR LBSHTMLAUTHDIR LBSTEMPLATEDIR LBSDATADIR LBSLOGDIR
export LBSTMPFSLOGDIR LBSCONFIGDIR LBSSBINDIR LBSBINDIR

# Plugin database path
PLUGINDATABASE="$LBSDATADIR/plugindatabase.json"
export PLUGINDATABASE

# Determine plugin directory from script path
_LB_SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[1]:-$0}")" && pwd)/$(basename "${BASH_SOURCE[1]:-$0}")"
_LB_REL_PATH="${_LB_SCRIPT_PATH#$LBHOMEDIR/}"

# Auto-detect plugin name from script path
_detect_plugin() {
    local relpath="$1"
    local IFS='/'
    read -ra parts <<< "$relpath"

    if [ "${parts[0]}" = "webfrontend" ] && [ "${parts[2]}" = "plugins" ] && [ -n "${parts[3]}" ]; then
        echo "${parts[3]}"
    elif [ "${parts[0]}" = "templates" ] && [ "${parts[1]}" = "plugins" ] && [ -n "${parts[2]}" ]; then
        echo "${parts[2]}"
    elif [ "${parts[0]}" = "log" ] && [ "${parts[1]}" = "plugins" ] && [ -n "${parts[2]}" ]; then
        echo "${parts[2]}"
    elif [ "${parts[0]}" = "data" ] && [ "${parts[1]}" = "plugins" ] && [ -n "${parts[2]}" ]; then
        echo "${parts[2]}"
    elif [ "${parts[0]}" = "config" ] && [ "${parts[1]}" = "plugins" ] && [ -n "${parts[2]}" ]; then
        echo "${parts[2]}"
    elif [ "${parts[0]}" = "bin" ] && [ "${parts[1]}" = "plugins" ] && [ -n "${parts[2]}" ]; then
        echo "${parts[2]}"
    elif [ "${parts[0]}" = "system" ] && [ "${parts[1]}" = "daemons" ] && [ "${parts[2]}" = "plugins" ] && [ -n "${parts[3]}" ]; then
        echo "${parts[3]}"
    fi
}

# Set plugin variables if we can detect the plugin context
if [ -z "$LBPPLUGINDIR" ]; then
    _DETECTED_PLUGIN=$(_detect_plugin "$_LB_REL_PATH")
    if [ -n "$_DETECTED_PLUGIN" ]; then
        LBPPLUGINDIR="$_DETECTED_PLUGIN"
    fi
fi

if [ -n "$LBPPLUGINDIR" ]; then
    LBPHTMLAUTHDIR="$LBHOMEDIR/webfrontend/htmlauth/plugins/$LBPPLUGINDIR"
    LBPHTMLDIR="$LBHOMEDIR/webfrontend/html/plugins/$LBPPLUGINDIR"
    LBPTEMPLATEDIR="$LBHOMEDIR/templates/plugins/$LBPPLUGINDIR"
    LBPDATADIR="$LBHOMEDIR/data/plugins/$LBPPLUGINDIR"
    LBPLOGDIR="$LBHOMEDIR/log/plugins/$LBPPLUGINDIR"
    LBPCONFIGDIR="$LBHOMEDIR/config/plugins/$LBPPLUGINDIR"
    LBPBINDIR="$LBHOMEDIR/bin/plugins/$LBPPLUGINDIR"

    export LBPPLUGINDIR LBPHTMLAUTHDIR LBPHTMLDIR LBPTEMPLATEDIR
    export LBPDATADIR LBPLOGDIR LBPCONFIGDIR LBPBINDIR
fi

# Clean up internal variables
unset _LB_SCRIPT_PATH _LB_REL_PATH _DETECTED_PLUGIN

########################################################################
# Utility functions
########################################################################

# Check if a value represents "enabled" (true, yes, on, 1, enable, enabled)
is_enabled() {
    local val
    val=$(echo "$1" | tr '[:upper:]' '[:lower:]')
    case "$val" in
        true|yes|on|1|enable|enabled) return 0 ;;
        *) return 1 ;;
    esac
}

# Check if a value represents "disabled" (false, no, off, 0, disable, disabled)
is_disabled() {
    local val
    val=$(echo "$1" | tr '[:upper:]' '[:lower:]')
    case "$val" in
        false|no|off|0|disable|disabled) return 0 ;;
        *) return 1 ;;
    esac
}

# Get current time in various formats
# Usage: currtime [format]
# Formats: hr (human readable), file (file-safe), iso (ISO 8601), epoch (unix timestamp)
currtime() {
    local fmt="${1:-hr}"
    case "$fmt" in
        hr)       date "+%d.%m.%Y %H:%M:%S" ;;
        file)     date "+%Y%m%d_%H%M%S" ;;
        iso)      date -Iseconds ;;
        epoch)    date +%s ;;
        hrtime)   date "+%H:%M:%S" ;;
        *)        date "+%d.%m.%Y %H:%M:%S" ;;
    esac
}

# Read a value from the general.json config
# Usage: lbsconfig_get <jsonpath>
# Example: lbsconfig_get '.Base.Lang'
lbsconfig_get() {
    local key="$1"
    if command -v jq >/dev/null 2>&1; then
        jq -r "$key // empty" "$LBSCONFIGDIR/general.json" 2>/dev/null
    fi
}

# Get LoxBerry system language
lblanguage() {
    local lang
    lang=$(lbsconfig_get '.Base.Lang')
    echo "${lang:-en}"
}

# Get LoxBerry hostname
lbhostname() {
    hostname 2>/dev/null || echo "loxberry"
}

# Get LoxBerry version
lbversion() {
    lbsconfig_get '.Base.Version'
}
