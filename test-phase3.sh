#!/bin/bash
# Phase 3 Testing Script - MQTT Gateway

set -e

BASE_URL="http://localhost:8080"

echo "=== Phase 3: MQTT Gateway Tests ==="
echo ""

# Test 1: Gateway status
echo "1. Testing GET /api/mqtt/status"
curl -s "$BASE_URL/api/mqtt/status" | jq '.'
echo ""

# Test 2: System status (should include MQTT info)
echo "2. Testing GET /api/system/status"
curl -s "$BASE_URL/api/system/status" | jq '.'
echo ""

echo "=== MQTT Gateway Integration Tests ==="
echo ""

# Check if mosquitto is running
echo "3. Checking Mosquitto broker"
docker ps | grep mosquitto && echo "✓ Mosquitto is running" || echo "✗ Mosquitto not found"
echo ""

# Check UDP port
echo "4. Checking UDP listener port"
nc -uz localhost 11884 && echo "✓ UDP port 11884 is open" || echo "✗ UDP port not accessible"
echo ""

echo "=== Manual Tests Required ==="
echo ""

echo "Subscribe to MQTT topics:"
echo "  1. Add subscriptions to volumes/config/system/mqtt_subscriptions.cfg:"
echo "     [Test1]"
echo "     TOPIC=home/#"
echo "     ENABLED=1"
echo ""
echo "  2. Reload subscriptions:"
echo "     curl -X POST $BASE_URL/api/mqtt/subscriptions/reload"
echo ""

echo "Test MQTT publishing:"
echo "  1. From host (if mosquitto-clients installed):"
echo "     mosquitto_pub -h localhost -t 'home/test' -m 'hello'"
echo ""
echo "  2. From mosquitto container:"
echo "     docker exec mosquitto mosquitto_pub -t 'home/test' -m 'hello'"
echo ""

echo "Test UDP input:"
echo "  1. JSON format:"
echo "     echo '{\"topic\":\"home/sensor\",\"value\":\"123\"}' | nc -u localhost 11884"
echo ""
echo "  2. Simple format:"
echo "     echo 'home/switch=1' | nc -u localhost 11884"
echo ""

echo "Test transformers:"
echo "  1. Boolean conversion:"
echo "     echo '{\"topic\":\"home/light\",\"value\":\"ON\"}' | nc -u localhost 11884"
echo "     # Should transform ON → 1"
echo ""

echo "Monitor logs:"
echo "  docker logs -f loxberry-rust | grep -i mqtt"
echo ""

echo "=== Phase 3 Basic Tests Complete ==="
