#!/bin/bash
# Phase 2 Testing Script - Plugin Management API

set -e

BASE_URL="http://localhost:8080"

echo "=== Phase 2: Plugin Management API Tests ==="
echo ""

# Test 1: List plugins (should be empty initially)
echo "1. Testing GET /api/plugins (list all plugins)"
curl -s "$BASE_URL/api/plugins" | jq '.'
echo ""

# Test 2: System status
echo "2. Testing GET /api/system/status"
curl -s "$BASE_URL/api/system/status" | jq '.'
echo ""

echo "=== Manual Tests Required ==="
echo ""
echo "To test plugin installation:"
echo "  1. Create a test plugin ZIP with:"
echo "     - plugin.cfg (with [AUTHOR], [PLUGIN] sections)"
echo "     - Optional hooks: preinstall.sh, postinstall.sh, uninstall.sh"
echo "     - Optional directories: webfrontend/, templates/, bin/, config/, data/"
echo ""
echo "  2. Install via API:"
echo "     curl -X POST -F 'file=@test-plugin.zip' $BASE_URL/api/plugins/install"
echo ""
echo "  3. List installed plugins:"
echo "     curl $BASE_URL/api/plugins"
echo ""
echo "  4. Get specific plugin (replace MD5):"
echo "     curl $BASE_URL/api/plugins/MD5_HASH"
echo ""
echo "  5. Uninstall plugin:"
echo "     curl -X DELETE $BASE_URL/api/plugins/MD5_HASH"
echo ""

echo "=== Phase 2 Basic Tests Complete ==="
