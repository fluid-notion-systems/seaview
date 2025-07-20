# Bevy Remote Protocol (BRP) Extras Integration

This document describes the BRP extras functionality integrated into Seaview.

## About bevy_brp_extras

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_extras.svg)](https://crates.io/crates/bevy_brp_extras)
[![Documentation](https://docs.rs/bevy_brp_extras/badge.svg)](https://docs.rs/bevy_brp_extras/)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp/extras#license)

bevy_brp_extras does two things:
1. Configures your app for bevy remote protocol (BRP)
2. Adds additional methods that can be used with BRP

## Features

The BrpExtrasPlugin adds the following Bevy Remote Protocol methods:

- `brp_extras/screenshot` - Capture screenshots of the primary window
- `brp_extras/shutdown` - Gracefully shutdown the application
- `brp_extras/discover_format` - Get correct data formats for BRP spawn/insert/mutation operations
- `brp_extras/send_keys` - Send keyboard input to the application
- `brp_extras/set_debug_mode` - Enable/disable debug information in format discovery

## Usage

The plugin is automatically added to Seaview and listens on BRP default port 15702.

### Custom Port

You can specify a custom port for the BRP server by setting the `BRP_PORT` environment variable:

```bash
BRP_PORT=8080 cargo run
```

## BRP Method Details

### Screenshot
- **Method**: `brp_extras/screenshot`
- **Parameters**:
  - `path` (string, required): File path where the screenshot should be saved
- **Returns**: Success status with the absolute path where the screenshot will be saved

**Note**: Screenshots require the `png` feature to be enabled in Bevy (already enabled in Seaview).

### Shutdown
- **Method**: `brp_extras/shutdown`
- **Parameters**: None
- **Returns**: Success status with shutdown confirmation

### Format Discovery
- **Method**: `brp_extras/discover_format`
- **Parameters**:
  - `types` (array of strings, required): **Fully-qualified component type paths** (e.g., `"bevy_transform::components::transform::Transform"`, not just `"Transform"`)
- **Returns**: Correct JSON structure needed for BRP spawn, insert, and mutation operations

**Example:**
```bash
curl -X POST http://localhost:15702/brp_extras/discover_format \
  -H "Content-Type: application/json" \
  -d '{"types": ["bevy_transform::components::transform::Transform", "bevy_core::name::Name"]}'
```

### Send Keys
- **Method**: `brp_extras/send_keys`
- **Parameters**:
  - `keys` (array of strings, required): Key codes to send (e.g., `["KeyA", "Space", "Enter"]`)
  - `duration_ms` (number, optional): How long to hold keys before releasing in milliseconds (default: 100, max: 60000)
- **Returns**: Success status with the keys sent and duration used

**Example:**
```bash
# Send "hi" by pressing H and I keys
curl -X POST http://localhost:15702/brp_extras/send_keys \
  -H "Content-Type: application/json" \
  -d '{"keys": ["KeyH", "KeyI"]}'

# Hold space key for 2 seconds
curl -X POST http://localhost:15702/brp_extras/send_keys \
  -H "Content-Type: application/json" \
  -d '{"keys": ["Space"], "duration_ms": 2000}'
```

### Set Debug Mode
- **Method**: `brp_extras/set_debug_mode`
- **Parameters**:
  - `enabled` (boolean, required): Enable or disable debug mode
- **Returns**: Success status with debug mode state

**Example:**
```bash
# Enable debug mode
curl -X POST http://localhost:15702/brp_extras/set_debug_mode \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

## Integration with Zed

The BRP extras plugin enables remote control of the Seaview application from external tools like Zed. With the BRP server running, you can:

1. Take screenshots of the rendered scene
2. Send keyboard input to navigate the camera
3. Discover component formats for scene manipulation
4. Control the application lifecycle

## Testing BRP Connection

To test if BRP is working, you can use curl to query the available methods:

```bash
# List all available BRP methods
curl -X POST http://localhost:15702/bevy/list
```

## Security Note

The BRP server listens on all interfaces by default. In production environments, consider:
- Restricting access to localhost only
- Using a firewall to limit access
- Running behind a reverse proxy with authentication

## License

The bevy_brp_extras crate is dual-licensed under either:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.