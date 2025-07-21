# Parallel STL Loading Research

## Executive Summary

This document outlines strategies for parallelizing STL file loading in the Seaview application to improve performance and prevent main thread blocking. The current implementation loads STL files synchronously on the main thread, which can cause UI freezes and poor user experience, especially with large files or sequences.

## Current Implementation Analysis

### Existing Architecture

1. **Synchronous Loading**: The `load_stl_file_optimized` function in `sequence/loader.rs` performs all STL parsing and mesh generation on the calling thread.

2. **Sequential Processing**: STL faces are processed one-by-one in a loop, with validation, normal calculation, and vertex generation happening sequentially.

3. **Preloading System**: The `preload_sequence_meshes` system attempts to load multiple files but does so synchronously within the Bevy system.

4. **Dependencies**: The project already includes `rayon` (v1.10) for parallel processing but doesn't currently use it for STL loading.

### Performance Bottlenecks

1. **File I/O**: Reading large STL files blocks the thread
2. **STL Parsing**: The `stl_io::read_stl` call is synchronous
3. **Face Processing**: Sequential iteration through potentially millions of faces
4. **Mesh Generation**: Building vertex/normal/UV arrays sequentially
5. **Asset Creation**: Creating Bevy mesh assets on the main thread

## Parallelization Strategies

### 1. Thread Pool for File Loading

**Approach**: Use a dedicated thread pool for STL file I/O and parsing.

```rust
use std::sync::mpsc;
use rayon::prelude::*;

pub struct ParallelStlLoader {
    sender: mpsc::Sender<LoadResult>,
    receiver: mpsc::Receiver<LoadResult>,
}

enum LoadResult {
    Success {
        path: PathBuf,
        mesh_data: MeshData,
        stats: FileLoadStats,
    },
    Error {
        path: PathBuf,
        error: String,
    },
}
```

**Benefits**:
- Non-blocking file I/O
- Multiple files can load simultaneously
- Main thread remains responsive

### 2. Parallel Face Processing

**Approach**: Use Rayon to parallelize face validation and vertex generation.

```rust
// Parallel face processing
let face_results: Vec<FaceResult> = stl.faces
    .par_iter()
    .enumerate()
    .map(|(idx, face)| process_face(idx, face, &stl.vertices))
    .collect();

// Collect results
let (valid_faces, positions, normals, uvs) = face_results
    .into_iter()
    .fold(/* accumulator */)
    .finish();
```

**Benefits**:
- Utilize all CPU cores for face processing
- Significant speedup for large meshes
- Better cache utilization with chunked processing

### 3. Async Asset Pipeline

**Approach**: Implement an async loading pipeline using Bevy's asset system.

```rust
pub struct AsyncStlLoader {
    loading_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    worker_handles: Vec<std::thread::JoinHandle<()>>,
}

impl AsyncStlLoader {
    pub fn queue_load(&self, path: PathBuf) -> LoadHandle {
        // Queue file for loading
        // Return handle for tracking
    }
    
    pub fn poll_completed(&mut self) -> Vec<CompletedLoad> {
        // Check for completed loads
        // Transfer to main thread
    }
}
```

**Benefits**:
- True async loading without blocking systems
- Progress tracking and cancellation support
- Integration with Bevy's asset lifecycle

### 4. Streaming Mesh Generation

**Approach**: Stream mesh data as it's generated rather than building complete arrays.

```rust
pub struct StreamingMeshBuilder {
    vertex_sender: mpsc::Sender<Vec<[f32; 3]>>,
    normal_sender: mpsc::Sender<Vec<[f32; 3]>>,
    uv_sender: mpsc::Sender<Vec<[f32; 2]>>,
}

impl StreamingMeshBuilder {
    pub fn process_chunk(&mut self, faces: &[Face]) {
        // Process chunk and send results
    }
}
```

**Benefits**:
- Lower memory pressure
- Earlier availability of partial data
- Better for very large files

## Implementation Recommendations

### Phase 1: Basic Thread Pool (Quick Win)

1. Implement a simple thread pool for file loading
2. Move `load_stl_file_optimized` execution to worker threads
3. Use channels to communicate results back to main thread
4. Update `MeshCache::get_or_load` to queue loads instead of blocking

**Estimated effort**: 2-3 days
**Performance gain**: 50-70% reduction in main thread blocking

### Phase 2: Parallel Face Processing

1. Refactor face processing loop to use Rayon
2. Implement chunked processing for better cache locality
3. Add progress reporting for large files
4. Optimize memory allocation strategies

**Estimated effort**: 3-4 days
**Performance gain**: 3-5x speedup for large meshes (>100k faces)

### Phase 3: Full Async Pipeline

1. Design async loader architecture
2. Implement worker thread management
3. Add cancellation support
4. Integrate with Bevy's asset system
5. Add priority queue for load ordering

**Estimated effort**: 1-2 weeks
**Performance gain**: Near-zero main thread impact

## Technical Considerations

### Memory Management

- Pre-allocate vectors based on face count
- Use memory pools for temporary allocations
- Consider memory-mapped files for very large STLs
- Implement streaming for files > 100MB

### Error Handling

- Graceful degradation for corrupted files
- Partial mesh loading support
- Timeout handling for stuck loads
- Progress reporting for user feedback

### Thread Safety

- Use Arc<Mutex<>> sparingly (prefer channels)
- Avoid shared mutable state
- Design for lock-free operations where possible
- Consider using crossbeam for advanced concurrency

### Platform Considerations

- Test on various CPU core counts (2-32 cores)
- Consider WASM limitations (no std::thread)
- Handle resource-constrained environments
- Implement adaptive parallelism based on system load

## Benchmarking Plan

### Test Cases

1. **Small files** (<1MB): Measure overhead
2. **Medium files** (1-50MB): Typical use case
3. **Large files** (50-500MB): Stress test
4. **Sequences**: Multiple file loading
5. **Mixed workload**: Various file sizes

### Metrics

- Main thread frame time
- Total load time
- Memory usage (peak and average)
- CPU utilization
- User-perceived responsiveness

### Tools

- Bevy's built-in diagnostics
- Tracy profiler integration
- Custom timing infrastructure
- Memory profilers (heaptrack, valgrind)

## Code Examples

### Example 1: Simple Thread Pool Implementation

```rust
use std::sync::{Arc, Mutex};
use std::thread;
use crossbeam_channel::{bounded, Sender, Receiver};

pub struct StlLoadRequest {
    pub path: PathBuf,
    pub config: LoadConfig,
}

pub struct StlLoadResult {
    pub path: PathBuf,
    pub result: Result<(Mesh, FileLoadStats), String>,
}

pub struct ParallelStlLoader {
    request_tx: Sender<StlLoadRequest>,
    result_rx: Receiver<StlLoadResult>,
    _workers: Vec<thread::JoinHandle<()>>,
}

impl ParallelStlLoader {
    pub fn new(num_workers: usize) -> Self {
        let (request_tx, request_rx) = bounded(100);
        let (result_tx, result_rx) = bounded(100);
        
        let mut workers = Vec::new();
        
        for _ in 0..num_workers {
            let rx = request_rx.clone();
            let tx = result_tx.clone();
            
            let handle = thread::spawn(move || {
                while let Ok(request) = rx.recv() {
                    let result = load_stl_file_optimized(&request.path);
                    let _ = tx.send(StlLoadResult {
                        path: request.path,
                        result: result.map_err(|e| e.to_string()),
                    });
                }
            });
            
            workers.push(handle);
        }
        
        Self {
            request_tx,
            result_rx,
            _workers: workers,
        }
    }
    
    pub fn queue_load(&self, path: PathBuf, config: LoadConfig) -> Result<(), &'static str> {
        self.request_tx
            .try_send(StlLoadRequest { path, config })
            .map_err(|_| "Load queue full")
    }
    
    pub fn poll_results(&self) -> Vec<StlLoadResult> {
        let mut results = Vec::new();
        while let Ok(result) = self.result_rx.try_recv() {
            results.push(result);
        }
        results
    }
}
```

### Example 2: Parallel Face Processing

```rust
use rayon::prelude::*;

fn process_faces_parallel(stl: &StlData) -> Result<MeshData, Box<dyn Error>> {
    // Process faces in parallel chunks
    const CHUNK_SIZE: usize = 1000;
    
    let face_chunks: Vec<_> = stl.faces
        .par_chunks(CHUNK_SIZE)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            let mut chunk_positions = Vec::with_capacity(chunk.len() * 3);
            let mut chunk_normals = Vec::with_capacity(chunk.len() * 3);
            let mut chunk_uvs = Vec::with_capacity(chunk.len() * 3);
            let mut stats = ChunkStats::default();
            
            for (local_idx, face) in chunk.iter().enumerate() {
                let face_idx = chunk_idx * CHUNK_SIZE + local_idx;
                
                if let Some(face_data) = process_single_face(face, &stl.vertices, face_idx) {
                    chunk_positions.extend_from_slice(&face_data.positions);
                    chunk_normals.extend_from_slice(&face_data.normals);
                    chunk_uvs.extend_from_slice(&face_data.uvs);
                    stats.valid_faces += 1;
                } else {
                    stats.skipped_faces += 1;
                }
            }
            
            ChunkResult {
                positions: chunk_positions,
                normals: chunk_normals,
                uvs: chunk_uvs,
                stats,
            }
        })
        .collect();
    
    // Combine chunks
    let total_vertices = face_chunks.iter()
        .map(|c| c.positions.len())
        .sum();
    
    let mut positions = Vec::with_capacity(total_vertices);
    let mut normals = Vec::with_capacity(total_vertices);
    let mut uvs = Vec::with_capacity(total_vertices);
    
    for chunk in face_chunks {
        positions.extend(chunk.positions);
        normals.extend(chunk.normals);
        uvs.extend(chunk.uvs);
    }
    
    Ok(MeshData {
        positions,
        normals,
        uvs,
    })
}
```

### Example 3: Integration with Bevy Systems

```rust
pub struct ParallelLoaderPlugin;

impl Plugin for ParallelLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ParallelStlLoader::new(num_cpus::get()))
            .add_systems(Update, (
                process_load_requests,
                handle_completed_loads,
            ).chain());
    }
}

fn process_load_requests(
    loader: Res<ParallelStlLoader>,
    mut mesh_cache: ResMut<MeshCache>,
    sequence_manager: Res<SequenceManager>,
) {
    // Queue loads for upcoming frames
    if let Some(sequence) = &sequence_manager.current_sequence {
        let current = sequence_manager.current_frame;
        let prefetch_range = current..=(current + 10).min(sequence.frame_count() - 1);
        
        for frame_idx in prefetch_range {
            if let Some(path) = sequence.frame_path(frame_idx) {
                if !mesh_cache.is_loaded(&path) && !mesh_cache.is_loading(&path) {
                    if loader.queue_load(path.clone(), LoadConfig::default()).is_ok() {
                        mesh_cache.mark_loading(path);
                    }
                }
            }
        }
    }
}

fn handle_completed_loads(
    loader: Res<ParallelStlLoader>,
    mut mesh_cache: ResMut<MeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut events: EventWriter<LoadingProgressEvent>,
) {
    for result in loader.poll_results() {
        match result.result {
            Ok((mesh, stats)) => {
                let handle = meshes.add(mesh);
                mesh_cache.insert_loaded(result.path, handle, stats);
                
                events.send(LoadingProgressEvent {
                    current: mesh_cache.loaded_count(),
                    total: mesh_cache.total_queued(),
                    percentage: mesh_cache.progress(),
                });
            }
            Err(error) => {
                error!("Failed to load {:?}: {}", result.path, error);
                mesh_cache.mark_failed(result.path);
            }
        }
    }
}
```

## Conclusion

Parallelizing STL file loading offers significant performance improvements for the Seaview application. The recommended approach is to implement changes in phases, starting with basic thread pool loading and progressively adding more sophisticated parallelization techniques.

Key benefits include:
- Responsive UI during loading
- Faster sequence preloading
- Better utilization of modern multi-core CPUs
- Scalability for larger datasets

The existing `rayon` dependency and modular architecture make these improvements feasible with moderate effort. Phase 1 implementation could be completed within a few days and would provide immediate user-visible improvements.