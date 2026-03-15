#!/bin/bash
# Sample postinstall hook
# This runs after plugin files are copied

echo "Running postinstall hook for sample plugin"
echo "Plugin directory: $LBPBINDIR"

# Create a sample data file
mkdir -p "$LBPDATADIR"
echo "Sample data content" > "$LBPDATADIR/sample.txt"

echo "Postinstall completed successfully"
exit 0
