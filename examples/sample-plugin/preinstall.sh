#!/bin/bash
# Sample preinstall hook
# This runs before plugin files are copied

echo "Running preinstall hook for sample plugin"
echo "LBHOMEDIR: $LBHOMEDIR"
echo "LBPPLUGINDIR: $LBPPLUGINDIR"
echo "LBPDATADIR: $LBPDATADIR"

# Create sample config file
mkdir -p "$LBPCONFIGDIR"
echo "installed_at=$(date)" > "$LBPCONFIGDIR/install.cfg"

echo "Preinstall completed successfully"
exit 0
