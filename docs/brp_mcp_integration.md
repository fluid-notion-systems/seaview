# BRP MCP Server Integration

## Overview

The `bevy_brp_mcp` server is a Model Context Protocol (MCP) server that provides comprehensive Bevy Remote Protocol (BRP) integration for AI coding assistants. It enables remote control, inspection, and mutation of Bevy applications.

## Available Tools

### Application Management

1. **brp_list_bevy_apps** - Discover Bevy applications in your workspace
   - Finds all Bevy apps with their build status
   - Identifies apps with BRP support

2. **brp_launch_bevy_app** - Launch a Bevy application
   - Starts apps with proper asset loading
   - Creates log files in `/tmp/`
   - Supports debug/release profiles

3. **brp_status** - Check if a Bevy app is running
   - Verifies process status
   - Checks BRP connectivity
   - Returns PID if running

### Entity Operations

4. **bevy_spawn** - Create new entities
   - Spawn entities with components
   - Uses JSON format for component data

5. **bevy_destroy** - Remove entities
   - Permanently destroys entities and their components

6. **bevy_query** - Query entities
   - Find entities by component filters
   - Retrieve component data
   - Supports complex queries

7. **bevy_get** - Get component data from entities
   - Retrieve specific components from an entity

8. **bevy_list** - List components or resources
   - List all registered components
   - List components on a specific entity

### Component Operations

9. **bevy_insert** - Add components to entities
   - Insert new components
   - Replace existing components

10. **bevy_remove** - Remove components from entities
    - Remove specific component types
    - Entity continues to exist

11. **bevy_mutate_component** - Modify component fields
    - Change specific fields without replacing entire component
    - Supports nested field paths

### Resource Operations

12. **bevy_get_resource** - Read global resources
    - Access application-wide state

13. **bevy_insert_resource** - Set global resources
    - Create or update resources

14. **bevy_remove_resource** - Delete global resources
    - Remove resources from the world

15. **bevy_mutate_resource** - Modify resource fields
    - Update specific fields in resources

16. **bevy_list_resources** - List all resources
    - Discover available resource types

### Hierarchy Operations

17. **bevy_reparent** - Change entity parent-child relationships
    - Set or remove entity parents
    - Reorganize entity hierarchies

### Monitoring & Debugging

18. **bevy_get_watch** - Monitor component changes
    - Watch specific components on an entity
    - Logs changes to file

19. **bevy_list_watch** - Monitor component additions/removals
    - Track structural changes to entities

20. **brp_list_active_watches** - List all active monitors
    - See running watch subscriptions

21. **brp_stop_watch** - Stop monitoring an entity
    - Clean up watch resources

### Log Management

22. **brp_list_logs** - List application log files
    - Find logs in temp directory
    - Filter by app name

23. **brp_read_log** - Read log file contents
    - View application output
    - Support for filtering and tailing

24. **brp_cleanup_logs** - Delete old log files
    - Clean up temp directory
    - Filter by age or app name

### Enhanced Features (requires bevy_brp_extras)

25. **brp_extras_screenshot** - Capture screenshots
    - Save current rendered frame
    - Returns image file path

26. **brp_extras_shutdown** - Gracefully shutdown app
    - Clean termination
    - Falls back to process kill if needed

27. **brp_extras_discover_format** - Get component JSON formats
    - Discover correct format for BRP operations
    - Essential for complex components

### Discovery & Introspection

28. **bevy_registry_schema** - Get type schema information
    - Retrieve JSON schemas for registered types
    - Filter by crate or type
    - **Warning**: Can return very large responses

29. **bevy_rpc_discover** - List available BRP methods
    - Get OpenRPC specification
    - Discover method signatures

## Usage with Shoreview

Since Shoreview already has BRP support via the RemotePlugin, you can:

1. Launch the app with a sequence directory:
   ```
   bevy_launch_app with app_name="shoreview"
   ```

2. Query loaded meshes:
   ```
   bevy_query to find entities with Mesh3d components
   ```

3. Monitor playback:
   ```
   bevy_get_watch on sequence manager entity
   ```

4. Take screenshots during playback:
   ```
   brp_extras_screenshot (requires bevy_brp_extras in app)
   ```

## Best Practices

1. **Always check app status** before sending BRP commands
2. **Use format discovery** for complex component operations
3. **Clean up logs** periodically to save disk space
4. **Stop watches** when done monitoring
5. **Filter registry schemas** to avoid token limits

## Troubleshooting

- **Timeout errors**: Some operations may timeout on large workspaces
- **Method not found**: Ensure the app has required plugins (RemotePlugin, BrpExtrasPlugin)
- **Format errors**: Use discover_format to get correct JSON structure
- **Connection refused**: Check app is running and BRP is enabled on port 15702