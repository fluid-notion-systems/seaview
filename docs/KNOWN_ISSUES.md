# Known Issues and Solutions

**Last Updated**: January 20, 2025

## Mesh Disappearing When Viewed Straight On

### Issue Description
**Symptom**: When looking directly at a mesh from certain angles, the mesh disappears or flickers out of view.

**Cause**: Backface culling. By default, Bevy (and most 3D engines) only render the "front" faces of triangles. When you look at a mesh from behind or when the coordinate transformation flips the winding order, the triangles are culled and become invisible.

### Root Causes

1. **Coordinate System Transformation**
   - Z-up to Y-up rotation can flip triangle winding order
   - What was "front" becomes "back" after transformation

2. **Inverted Normals in Source Data**
   - Some mesh exporters create meshes with inverted normals
   - Common with CAD exports and certain file formats

3. **Single-Sided Rendering Default**
   - Bevy's `StandardMaterial` defaults to front-face culling only
   - This is correct for closed meshes but problematic for:
     - Open/sheet meshes (fluid surfaces, terrain)
     - Meshes with inconsistent winding
     - Transformed meshes

### Solution Implemented ✅

**Location**: `crates/seaview/src/lib/sequence/loader.rs:166`

Set `cull_mode: None` on materials to enable double-sided rendering:

```rust
let material = materials.add(StandardMaterial {
    base_color: Color::srgb(0.3, 0.5, 0.8),
    perceptual_roughness: 0.4,
    metallic: 0.1,
    cull_mode: None, // ← Double-sided rendering
    ..default()
});
```

Additionally, a system (`ensure_double_sided_materials`) automatically enables double-sided rendering for all loaded mesh materials.

### Performance Impact

**Minimal**: Double-sided rendering means rendering ~2x triangles, but:
- Modern GPUs handle this efficiently
- Most mesh viewers use double-sided by default
- Critical for scientific/engineering visualization
- User can toggle if needed (future enhancement)

### Alternative Solutions (Not Implemented)

1. **Fix Mesh Normals at Load Time**
   - ❌ Modifies source data
   - ❌ CPU-intensive
   - ❌ May not fix coordinate transform issues

2. **Flip Winding Order**
   - ❌ Complex to implement correctly
   - ❌ Doesn't handle mixed winding in same mesh
   - ❌ Still need double-sided for open meshes

3. **Camera-Based Culling**
   - ❌ Doesn't work when moving camera
   - ❌ Unreliable with transformations

### Testing

Test with different viewing angles:
```bash
cargo run -- assets/test_models/Duck.glb

# Move camera around mesh
# WASD - move
# Mouse - look around
# Should never see mesh disappear
```

### Future Enhancements

- [ ] UI toggle for cull mode (Front/Back/None)
- [ ] Per-sequence cull mode override
- [ ] Automatic detection of closed vs open meshes
- [ ] Normal visualization mode for debugging

---

## Asset Loading from Absolute Paths

### Issue Description
**Symptom**: Frames fail to load with `AssetReaderError(NotFound(...))`

**Cause**: Bevy's `AssetServer` requires pre-registered asset sources. Can't load from arbitrary paths directly.

### Solution Implemented ✅

**Location**: `crates/seaview/src/main.rs:46-80`

Register "seq://" asset source at app startup:

```rust
// Canonicalize path to get absolute path
let canonical_path = path.canonicalize()?;

// Register as asset source before adding plugins
app.register_asset_source(
    "seq",
    AssetSourceBuilder::platform_default(&path_str, None)
);

// Then load with seq:// protocol
asset_server.load("seq://frame_0000.glb")
```

### Key Points

- Asset sources must be registered **before** `add_plugins(DefaultPlugins)`
- Use `canonicalize()` to get absolute path
- Load with `"seq://filename"` not full path
- One asset source per sequence directory

---

## Coordinate System Transformations

### Issue Description
**Symptom**: Mesh appears rotated or oriented incorrectly

**Cause**: Source data uses different coordinate system than Bevy (Y-up)

### Solution Implemented ✅

Use `--source-coordinates` CLI flag:

```bash
# Y-up (Bevy/graphics default)
cargo run -- --source-coordinates=yup /path

# Z-up (CAD/GIS/scientific)
cargo run -- --source-coordinates=zup /path

# FluidX3D (CFD)
cargo run -- --source-coordinates=fluidx3d /path
```

Transformation applied at entity level (GPU-accelerated, preserves data).

**Documentation**: See `docs/COORDINATE_SYSTEM_STATUS.md`

---

## Recursive Directory Scanning

### Issue Description
**Symptom**: Loading takes a long time, finds unexpected files in subdirectories

**Cause**: Discovery was set to `recursive: true` by default

### Solution Implemented ✅

**Location**: `crates/seaview/src/main.rs:183`

Changed to `recursive: false`:
```rust
DiscoverSequenceRequest {
    directory: path.clone(),
    recursive: false, // Only scan specified directory
    source_orientation: *source_orientation,
}
```

Now only scans the provided directory, not subdirectories.

---

## Performance Considerations

### Large Sequences (1000+ Frames)

**Current Behavior**: All frames loaded into memory simultaneously

**Impact**:
- High memory usage
- Long initial load time
- May cause OOM on large sequences

**Future Solutions** (Not Implemented):
- [ ] Streaming loader (sliding window cache)
- [ ] On-demand loading (load around current frame)
- [ ] Frame unloading (LRU cache)
- [ ] Memory limit configuration

**Workaround**: Split large sequences into smaller chunks

### Hot Reloading

**Status**: Not currently enabled

**To Enable**: Add `file_watcher` feature to Bevy in `Cargo.toml`:
```toml
bevy = { features = ["file_watcher"] }
```

**Note**: Useful for development but may impact performance

---

## Reporting Issues

When reporting issues, please include:

1. **Command used**: Full command line with all flags
2. **File information**: 
   - File format (GLB/glTF/STL)
   - File size
   - Source application that created it
3. **Behavior**: Expected vs actual
4. **Logs**: Run with `RUST_LOG=seaview=debug`
5. **System**: OS, GPU, RAM

Example:
```bash
RUST_LOG=seaview=debug cargo run -- --source-coordinates=zup /path/to/sequence 2>&1 | tee debug.log
```

---

## Additional Resources

- **Asset Loading**: `docs/research/ASSET_LOADING_IMPLEMENTATION.md`
- **Coordinate Systems**: `docs/research/coordinate_systems.md`
- **Debugging**: `docs/research/DEBUGGING_GUIDE.md`
