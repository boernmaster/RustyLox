#!/bin/bash
# Sample uninstall hook
# This runs before plugin directories are removed

echo "Running uninstall hook for sample plugin"
echo "Cleaning up plugin data..."

# Perform any custom cleanup here
# Note: Plugin directories will be automatically removed after this hook

echo "Uninstall hook completed successfully"
exit 0
