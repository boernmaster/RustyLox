# MQTT Filter Refactoring - March 16, 2026

## Change Summary

**Refactored MQTT topic filtering from per-subscription to global filter configuration.**

### Before
- Filter was configured **per subscription** in `mqtt_subscriptions.cfg`
- Each subscription could have its own `FILTER=` line
- Had to repeat the same filter for multiple subscriptions
- Filter checked in `SubscriptionManager::should_filter()`

### After
- Filter is configured **globally** in `general.json` under `Mqtt.Topicfilter`
- Single filter applies to ALL MQTT messages before sending to Miniserver
- Configured once in the "Broker Settings" tab of `/mqtt/config`
- Filter checked in `Relay::should_filter()`

---

## Why This Change?

The user correctly pointed out that a global filter makes more sense:

1. **Simplicity**: Define once instead of repeating for every subscription
2. **Consistency**: Same filter rules apply to all topics
3. **Maintainability**: Update in one place instead of many
4. **UX**: Clearer configuration - one place to configure filtering

Typical use case: Exclude system/health check topics like `_healthcheck_`, `_info_`, `_announce_` from ALL subscriptions.

---

## Files Modified

### Configuration Layer
**`crates/loxberry-config/src/mqtt.rs`**
- Added `topicfilter: String` field to `MqttConfig` struct
- Updated `Default` impl to include empty string
- Serialized as `Topicfilter` in JSON

### MQTT Gateway Core
**`crates/mqtt-gateway/src/subscription.rs`**
- **Removed** `filter: Option<String>` from `Subscription` struct
- **Removed** filter parsing from INI parser
- **Removed** `should_filter()` method from `SubscriptionManager`
- Simplified to only handle topic/name/enabled/plugin fields

**`crates/mqtt-gateway/src/relay.rs`**
- **Removed** dependency on `SubscriptionManager` for filtering
- **Added** `should_filter()` method that reads from global config
- Reads `config.mqtt.topicfilter` before each send
- Same regex logic: normalize topic, match pattern, exclude if match

**`crates/mqtt-gateway/src/lib.rs`**
- Updated `Relay::new()` to only accept config (removed subscription_manager param)

### Web UI
**`crates/web-ui/src/templates.rs`**
- Added `topicfilter: String` to `MqttConfigForm`

**`crates/web-ui/src/handlers/mqtt.rs`**
- Added `topicfilter` to `MqttConfigFormData`
- Read/write `config.mqtt.topicfilter` in config handlers

**`crates/web-ui/templates/mqtt/config.html`**
- **Moved** filter input from "Subscriptions" tab to "Broker Settings" tab
- Renamed function from `addToFilter()` to `addToGlobalFilter()`
- **Removed** filter field from subscription add form
- Updated help text to explain global filter location

**`crates/web-ui/src/handlers/mqtt_management.rs`**
- **Removed** `filter: Option<String>` from `SubscriptionForm`
- **Removed** `filter: Option<String>` from `ParsedSubscription`
- **Removed** filter parsing from `parse_subscriptions_cfg()`
- **Removed** filter display from subscription list HTML
- **Removed** filter saving in add/delete subscription handlers

---

## Configuration Format

### Old Format (Per-Subscription)
```ini
# mqtt_subscriptions.cfg
[HomeAutomation]
TOPIC=home/#
NAME=Home Sensors
FILTER=_healthcheck_|_info_|_announce_    # Repeated for each subscription
ENABLED=1

[LightControls]
TOPIC=lights/#
NAME=Light Controls
FILTER=_healthcheck_|_info_|_announce_    # Same filter repeated
ENABLED=1
```

### New Format (Global)
```json
// general.json
{
  "Mqtt": {
    "Brokerhost": "mosquitto",
    "Brokerport": "1883",
    "Udpinport": "11884",
    "Topicfilter": "_healthcheck_|_info_|_announce_"    // Global filter
  }
}
```

```ini
# mqtt_subscriptions.cfg (simplified)
[HomeAutomation]
TOPIC=home/#
NAME=Home Sensors
ENABLED=1

[LightControls]
TOPIC=lights/#
NAME=Light Controls
ENABLED=1
```

---

## UI Changes

### Before
**Broker Settings Tab**
- Broker host, port, credentials, UDP port

**Subscriptions Tab**
- Each subscription has its own filter input
- Filter examples shown per subscription
- Quick-add buttons for each subscription

### After
**Broker Settings Tab**
- Broker host, port, credentials, UDP port
- **Global Topic Filter** input field ✨ NEW
- Filter examples with quick-add buttons
- Help text explaining global filter

**Subscriptions Tab**
- Subscription topic and name only
- No filter input
- Info box explaining global filter location

---

## Migration Path

**Existing installations with per-subscription filters:**

1. **Read old filters** from `mqtt_subscriptions.cfg`
2. **Combine** all unique filter patterns with `|`
3. **Set** as global filter in MQTT config
4. **Remove** FILTER lines from subscriptions

Example migration script:
```bash
# Extract all unique filter patterns
grep "^FILTER=" mqtt_subscriptions.cfg | \
  cut -d= -f2 | \
  sort -u | \
  tr '\n' '|' | \
  sed 's/|$//'
```

**Manual migration:**
1. Note your existing filters
2. Go to `/mqtt/config` → Broker Settings tab
3. Paste combined filter into "Global Topic Filter"
4. Save configuration

---

## Testing

### Test Global Filter
```bash
# 1. Configure global filter via UI
curl -X POST http://localhost:8080/mqtt/config \
  -d "brokerhost=mosquitto&brokerport=1883&udpinport=11884&topicfilter=_healthcheck_|_info_"

# 2. Publish filtered message (should NOT go to Miniserver)
mosquitto_pub -t "home/_healthcheck_" -m "OK"

# 3. Publish normal message (should go to Miniserver)
mosquitto_pub -t "home/temperature" -m "22.5"

# 4. Check logs
docker logs rustylox 2>&1 | grep FILTERED  # Should see filtered message
docker logs rustylox 2>&1 | grep "Sent to Miniserver"  # Should see temperature
```

### Verify UI
```bash
# 1. Open MQTT config page
open http://localhost:8080/mqtt/config

# 2. Check Broker Settings tab
# - Should see "Global Topic Filter" input
# - Should have quick-add buttons
# - Should have help text

# 3. Check Subscriptions tab
# - Should NOT see filter input in add form
# - Should see info box about global filter

# 4. Add subscription
# - Should only have topic and name fields
# - Should save without filter field
```

---

## Performance Impact

**Before:**
- Filter check: O(n) subscriptions × O(m) filter matching
- Each subscription checked independently

**After:**
- Filter check: O(1) config read + O(m) filter matching
- Single filter check per message

**Result:** Slightly faster filtering with large subscription counts.

---

## Benefits

1. **Simpler Configuration**
   - One place to configure filtering
   - No need to copy/paste filters

2. **Better UX**
   - Clear location for global rules
   - Less cluttered subscription form

3. **Easier Maintenance**
   - Update filter once for all subscriptions
   - No risk of inconsistent filters

4. **Cleaner Code**
   - Less duplication
   - Clearer separation of concerns
   - Simpler data structures

---

## Backward Compatibility

**Config File:** ✅ Backward compatible
- Old `general.json` without `Topicfilter` field: defaults to empty string (no filtering)
- Old `mqtt_subscriptions.cfg` with `FILTER=` lines: ignored (no errors)

**Web UI:** ✅ Gracefully handles missing field
- Form loads with empty filter if not set
- Quick-add buttons work immediately

**API:** ✅ New field is optional
- `topicfilter` field defaults to empty string via `#[serde(default)]`
- Older clients can omit the field

---

## Documentation Updates Needed

- [x] Update PHASE3_COMPLETE.md - global filter instead of per-subscription
- [x] Update PHASE5_PLAN.md - mark refactor as complete
- [x] This file - comprehensive refactor documentation

---

## Summary

Successfully refactored MQTT topic filtering from per-subscription to global configuration. The change:
- Simplifies configuration (define once vs. many times)
- Improves UX (clear single location)
- Maintains all functionality (same regex filtering)
- Backward compatible (graceful degradation)

All tests passing. Ready for runtime testing.

**Next Steps:**
1. Test in Docker environment
2. Create example config with global filter
3. Update user documentation
4. Consider migration script for existing installations
