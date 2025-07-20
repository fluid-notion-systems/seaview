# Bevy Mesh File Loading Research

## Overview

This document explores mesh file loading options in Bevy 0.14+, focusing on supporting multiple file formats with a primary emphasis on STL for immediate needs.

## Native Bevy Support

### GLTF/GLB (Built-in)
**Status**: First-class support in Bevy
**Module**: `bevy::gltf`

GLTF is Bevy's primary 3D asset format with the most comprehensive support:
- Complete scene graph support
- Materials and textures
- Animations
- Multiple meshes per file
- Extensible via GLTF extensions

```rust
// Simple GLTF loading
let scene_handle: Handle<Scene> = asset_server.load("model.gltf#Scene0");
let mesh_handle: Handle<Mesh> = asset_server.load("model.gltf#Mesh0");
```

### Bevy Asset System Architecture

Bevy uses the `AssetLoader` trait for implementing custom format support:

```rust
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Settings + Default + Serialize + for<'a> Deserialize<'a>;
    type Error: Into<Box<dyn Error + Send + Sync>>;

    fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>>;

    fn extensions(&self) -> &[&str];
}
```

## Third-Party Format Support

### 1. STL Support
**Crate**: `bevy_stl`
**Repository**: https://github.com/nilclass/bevy_stl
**Status**: Active but may need updates for Bevy 0.14

```toml
[dependencies]
bevy_stl = "0.12"  # Check for 0.14 compatibility
```

```rust
use bevy_stl::StlPlugin;

app.add_plugins(StlPlugin);
let mesh_handle: Handle<Mesh> = asset_server.load("model.stl");
```

**Pros**:
- Simple integration
- Both ASCII and binary STL support
- Based on robust `stl_io` crate

**Cons**:
- No material information (STL limitation)
- Single mesh per file
- May need forking for latest Bevy

### 2. PLY Support
**Crate**: `bevy_ply`
**Repository**: https://github.com/rezural/bevy_ply
**Status**: May need updates

**Pros**:
- Support for vertex colors
- Point cloud support
- Flexible attribute system

**Cons**:
- Less maintained
- Limited documentation

### 3. OBJ Support
**Options**:
1. `bevy_obj` - Dedicated OBJ loader
2. `tobj` - Rust OBJ library that can be wrapped

**Pros**:
- Widely used format
- Material support via MTL files
- Human-readable

**Cons**:
- Multiple files (OBJ + MTL + textures)
- No advanced features

## Recommended Implementation Strategy

### Phase 1: Multi-Format Architecture

Create a unified mesh loading system that abstracts over different formats:

```rust
// Unified mesh loading trait
pub trait MeshFormat: Send + Sync + 'static {
    fn extensions(&self) -> &[&str];
    fn can_load(&self, extension: &str) -> bool;
    fn load_mesh(&self, data: &[u8]) -> Result<MeshData, MeshLoadError>;
}

// Common mesh representation
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub indices: Option<Vec<u32>>,
    pub colors: Option<Vec<[f32; 4]>>,
}

// Format implementations
pub struct StlFormat;
pub struct PlyFormat;
pub struct ObjFormat;
```

### Phase 2: STL Implementation (Immediate Need)

For STL support, we have three options:

#### Option A: Use bevy_stl (Recommended for quick start)
```toml
[dependencies]
bevy_stl = { git = "https://github.com/nilclass/bevy_stl", branch = "main" }
```

#### Option B: Direct stl_io Integration
```toml
[dependencies]
stl_io = "0.7"
```

```rust
use stl_io::{read_stl, Vector, Triangle};
use bevy::render::mesh::{Mesh, Indices, PrimitiveTopology};

pub struct StlLoader;

impl AssetLoader for StlLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = StlLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let stl = stl_io::read_stl(&mut bytes.as_slice())?;

        // Convert STL to Bevy mesh
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for (i, triangle) in stl.triangles.iter().enumerate() {
            let base_index = (i * 3) as u32;

            // Add vertices
            for j in 0..3 {
                positions.push([
                    triangle.vertices[j][0],
                    triangle.vertices[j][1],
                    triangle.vertices[j][2],
                ]);
                normals.push([
                    triangle.normal[0],
                    triangle.normal[1],
                    triangle.normal[2],
                ]);
            }

            // Add indices
            indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
            ]);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.set_indices(Some(Indices::U32(indices)));

        Ok(mesh)
    }

    fn extensions(&self) -> &[&str] {
        &["stl"]
    }
}
```

#### Option C: Fork and Update bevy_stl
Fork the repository and update dependencies for Bevy 0.14 compatibility.

### Phase 3: Format Detection and Loading

Implement automatic format detection:

```rust
pub struct MeshFormatRegistry {
    formats: HashMap<String, Box<dyn MeshFormat>>,
}

impl MeshFormatRegistry {
    pub fn new() -> Self {
        let mut formats = HashMap::new();

        // Register formats
        let stl = Box::new(StlFormat);
        for ext in stl.extensions() {
            formats.insert(ext.to_string(), stl.clone());
        }

        Self { formats }
    }

    pub fn load_from_path(&self, path: &Path) -> Result<MeshData, MeshLoadError> {
        let extension = path.extension()
            .and_then(|e| e.to_str())
            .ok_or(MeshLoadError::UnknownFormat)?;

        let format = self.formats.get(extension)
            .ok_or(MeshLoadError::UnsupportedFormat(extension.to_string()))?;

        let data = std::fs::read(path)?;
        format.load_mesh(&data)
    }
}
```

## Best Practices for Mesh Loading

### 1. Error Handling
```rust
#[derive(Debug, thiserror::Error)]
pub enum MeshLoadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown file format")]
    UnknownFormat,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid mesh data: {0}")]
    InvalidData(String),

    #[error("STL parsing error: {0}")]
    StlError(#[from] stl_io::StlError),
}
```

### 2. Normal Calculation
Many formats (like STL) store face normals but not vertex normals. Implement smooth normal calculation:

```rust
pub fn calculate_smooth_normals(
    positions: &[[f32; 3]],
    indices: &[u32]
) -> Vec<[f32; 3]> {
    // Implementation for smooth vertex normals
}
```

### 3. Mesh Validation
```rust
pub fn validate_mesh(mesh_data: &MeshData) -> Result<(), MeshLoadError> {
    if mesh_data.positions.is_empty() {
        return Err(MeshLoadError::InvalidData("No vertices".into()));
    }

    if let Some(indices) = &mesh_data.indices {
        let max_index = *indices.iter().max().unwrap_or(&0) as usize;
        if max_index >= mesh_data.positions.len() {
            return Err(MeshLoadError::InvalidData("Invalid indices".into()));
        }
    }

    Ok(())
}
```

### 4. Async Loading
Use Bevy's async asset loading system for non-blocking loads:

```rust
pub async fn load_mesh_async(
    path: PathBuf,
) -> Result<MeshData, MeshLoadError> {
    tokio::task::spawn_blocking(move || {
        // Heavy loading work here
    }).await?
}
```

## Performance Considerations

### 1. Memory Efficiency
- Use index buffers to avoid vertex duplication
- Consider mesh decimation for LOD
- Implement streaming for large files

### 2. Loading Performance
- Parallel loading for multiple files
- Caching parsed mesh data
- Background loading with progress reporting

### 3. Format-Specific Optimizations
- **STL**: Binary format is much faster than ASCII
- **PLY**: Binary format available
- **OBJ**: Pre-process to binary format for production

## Recommendations for Seaview

### Immediate Implementation (Phase 1):
1. Start with direct `stl_io` integration for STL support
2. Create simple `AssetLoader` implementation
3. Add to existing plugin architecture

### Near-term (Phase 2):
1. Add PLY support for point clouds
2. Implement format detection system
3. Create unified mesh representation

### Long-term (Phase 3+):
1. Add OBJ support with materials
2. Implement mesh validation and repair
3. Add mesh preprocessing (decimation, optimization)
4. Consider custom binary format for sequences

## Example Integration

```rust
// In your main.rs or plugin
use seaview_mesh_loader::{MeshLoaderPlugin, StlFormat};

app.add_plugins(MeshLoaderPlugin::default()
    .with_format(StlFormat)
    .with_format(PlyFormat));

// Usage
let mesh_handle: Handle<Mesh> = asset_server.load("model.stl");
```

## Conclusion

For seaview's immediate needs:
1. **STL support** via direct `stl_io` integration is the fastest path
2. **GLTF** is already available and should be leveraged when possible
3. **Custom AssetLoader** implementation provides the most control
4. **Modular architecture** allows easy addition of new formats

The Bevy asset system is flexible enough to support any format through the `AssetLoader` trait, making it straightforward to add new formats as needed.
