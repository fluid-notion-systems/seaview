# Baby Shark Mesh Format Conversion Research

## Overview

This document explores the baby_shark library's mesh format conversion capabilities, specifically its ability to convert from polygon soup (non-indexed mesh data with duplicated vertices) to indexed mesh formats. This functionality is crucial for fixing mesh rendering issues in the Seaview project.

## Baby Shark Library Overview

**Repository**: Located in `vendor/baby_shark`  
**Purpose**: Geometry processing library in pure Rust  
**Key Features**:
- Implicit/voxel/volume modeling
- Boolean operations and offsetting
- Mesh simplification and remeshing
- **Polygon soup to indexed mesh conversion**

## Mesh Format Types in Baby Shark

### 1. PolygonSoup
A non-indexed mesh representation where each triangle stores its own three vertices:
- **Storage**: `Vec<Vec3<S>>` where every 3 vectors form a triangle
- **Characteristics**: 
  - Simple to generate
  - High memory usage due to vertex duplication
  - No vertex sharing between triangles
  - Common output format from marching cubes and other algorithms

### 2. CornerTable
An indexed mesh representation with unique vertices and face indices:
- **Storage**: Separate vertex and face arrays
- **Features**:
  - Efficient memory usage
  - Supports vertex attributes
  - Enables topological operations
  - Half-edge-like connectivity information

## Conversion Process

### Core Conversion Function
Baby_shark provides automatic conversion through the `FromSoup` trait:

```rust
impl<S: RealNumber> FromSoup for CornerTable<S> {
    type Scalar = S;

    fn from_triangles_soup(triangles: impl Iterator<Item = Vec3<Self::Scalar>>) -> Self {
        let indexed = merge_points(triangles);
        Self::from_vertex_and_face_slices(&indexed.points, &indexed.indices)
    }
}
```

### The merge_points Algorithm
The key to conversion is the `merge_points` function:
- **Purpose**: Removes duplicate vertices and creates index mapping
- **Method**: Uses a spatial hash map to identify coincident vertices
- **Output**: `IndexedVertices` struct containing:
  - `points`: Unique vertex positions
  - `indices`: Face indices referencing the unique vertices

## Integration Strategy for Seaview

### Current Problem
The mesh_sender_test is generating polygon soup data which causes:
- Inefficient memory usage
- Potential normal calculation issues
- Rendering artifacts due to vertex duplication

### Solution Approach

1. **Convert polygon soup to CornerTable**:
   ```rust
   use baby_shark::mesh::corner_table::CornerTableF;
   use baby_shark::mesh::traits::FromSoup;
   
   // Convert vertices iterator to CornerTable
   let corner_table = CornerTableF::from_triangles_soup(vertices.into_iter());
   ```

2. **Extract indexed data**:
   ```rust
   // Get unique vertices
   let vertices: Vec<Vec3<f32>> = corner_table.vertices()
       .map(|v| v.position().clone())
       .collect();
   
   // Get face indices
   let indices: Vec<u32> = corner_table.faces()
       .flat_map(|face| {
           let (v1, v2, v3) = corner_table.face_vertices(face);
           vec![v1.0, v2.0, v3.0]
       })
       .collect();
   ```

3. **Send via Bevy Remote Protocol**:
   - Send unique vertices array
   - Send face indices array
   - Ensure proper mesh component structure

## Benefits of Conversion

### Memory Efficiency
- Reduces vertex data by ~66% for typical meshes
- Smaller data transfer over BRP
- Better GPU memory utilization

### Rendering Quality
- Proper vertex normal calculation
- Eliminates z-fighting from duplicate vertices
- Enables smooth shading

### Performance
- Faster mesh updates
- Reduced bandwidth requirements
- Better cache coherency

## Implementation Considerations

### 1. Coordinate System
Ensure consistent coordinate system between:
- Baby_shark's Vec3 representation
- Bevy's coordinate system
- The mesh generation algorithm

### 2. Normal Calculation
After conversion, consider:
- Using baby_shark's normal calculation utilities
- Or letting Bevy calculate normals from the indexed mesh
- Ensuring consistent winding order

### 3. Error Handling
Handle edge cases:
- Empty mesh data
- Degenerate triangles
- Extremely large meshes

### 4. Performance Optimization
For real-time updates:
- Consider caching the CornerTable
- Reuse vertex buffers when possible
- Profile the conversion overhead

## Code Example

```rust
use baby_shark::{
    mesh::corner_table::CornerTableF,
    mesh::traits::{FromSoup, FromIndexed},
    helpers::aliases::Vec3f,
};

fn convert_polygon_soup_to_indexed(
    soup_vertices: Vec<[f32; 3]>
) -> (Vec<[f32; 3]>, Vec<u32>) {
    // Convert arrays to Vec3f
    let vertices_iter = soup_vertices
        .into_iter()
        .map(|[x, y, z]| Vec3f::new(x, y, z));
    
    // Create CornerTable from soup
    let corner_table = CornerTableF::from_triangles_soup(vertices_iter);
    
    // Extract unique vertices
    let unique_vertices: Vec<[f32; 3]> = corner_table
        .vertices()
        .map(|v| {
            let pos = v.position();
            [pos.x, pos.y, pos.z]
        })
        .collect();
    
    // Extract face indices
    let indices: Vec<u32> = corner_table
        .faces()
        .flat_map(|face| {
            let (v1, v2, v3) = corner_table.face_vertices(face);
            vec![v1.0 as u32, v2.0 as u32, v3.0 as u32]
        })
        .collect();
    
    (unique_vertices, indices)
}
```

## Testing Strategy

1. **Unit Tests**:
   - Test conversion with known geometry (cube, sphere)
   - Verify vertex count reduction
   - Check index validity

2. **Visual Tests**:
   - Compare rendered output before/after conversion
   - Check for normal artifacts
   - Verify mesh integrity

3. **Performance Tests**:
   - Measure conversion time
   - Compare memory usage
   - Profile BRP transmission time

## Conclusion

Baby_shark's mesh format conversion provides a robust solution for converting polygon soup to indexed mesh data. By integrating this functionality into the mesh_sender_test, we can:

1. Fix the current rendering issues
2. Improve performance and memory usage
3. Enable more advanced mesh operations in the future

The library's clean API and efficient implementation make it an ideal choice for this conversion task. The next step is to implement this conversion in the mesh_sender_test and verify that it resolves the mesh rendering issues.