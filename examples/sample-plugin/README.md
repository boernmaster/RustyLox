# Sample Plugin

<div align="center">

![Status](https://img.shields.io/badge/Status-Test%20Plugin-informational)
![Type](https://img.shields.io/badge/Type-Example-yellow)

</div>

This is a sample plugin for testing the LoxBerry Rust plugin manager.

## Structure

```
sample-plugin/
├── plugin.cfg              # Plugin configuration (required)
├── preinstall.sh          # Pre-installation hook
├── postinstall.sh         # Post-installation hook
├── uninstall.sh           # Uninstall hook
└── webfrontend/
    └── htmlauth/
        └── index.html     # Plugin web interface
```

## Creating the Test ZIP

From the `examples/sample-plugin/` directory, run:

```bash
zip -r ../sample-plugin.zip .
```

This creates `sample-plugin.zip` in the `examples/` directory.

## Installing via API

```bash
curl -X POST -F 'file=@examples/sample-plugin.zip' http://localhost:8080/api/plugins/install
```

## Testing the Installation

1. List installed plugins:
   ```bash
   curl http://localhost:8080/api/plugins | jq '.'
   ```

2. Get plugin details (replace MD5 with actual hash from list response):
   ```bash
   curl http://localhost:8080/api/plugins/MD5_HASH | jq '.'
   ```

3. Verify directories were created:
   ```bash
   docker exec rustylox ls -la /opt/loxberry/data/plugins/sampleplugin
   docker exec rustylox ls -la /opt/loxberry/config/plugins/sampleplugin
   docker exec rustylox ls -la /opt/loxberry/webfrontend/htmlauth/plugins/sampleplugin
   ```

4. Check plugin database:
   ```bash
   docker exec rustylox cat /opt/loxberry/data/system/plugindatabase.json | jq '.'
   ```

## Uninstalling

```bash
curl -X DELETE http://localhost:8080/api/plugins/MD5_HASH
```

## Expected Behavior

### During Installation:
1. ZIP is extracted to temp directory
2. plugin.cfg is parsed
3. MD5 checksum is calculated
4. preinstall.sh hook is executed
5. Plugin directories are created
6. Files are copied to isolated directories
7. postinstall.sh hook is executed
8. Plugin entry is added to database

### During Uninstallation:
1. Plugin is found in database
2. uninstall.sh hook is executed
3. Plugin directories are removed
4. Plugin entry is removed from database
