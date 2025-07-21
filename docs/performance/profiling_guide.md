# Performance Profiling Guide for Seaview

This guide covers the best tools and techniques for profiling performance issues in Bevy applications, specifically for diagnosing the FPS instability and loading performance problems.

## Quick Recommendations

For your specific issue (unstable FPS with large mesh loading), I recommend this approach in order:

1. **Tracy** - Best for frame-by-frame analysis and GPU/CPU timeline visualization
2. **Bevy's built-in diagnostics** - Already added, good for basic metrics
3. **puffin** - Easier to integrate than Tracy, good for Bevy-specific profiling
4. **cargo-flamegraph** - Best for CPU bottleneck identification

## 1. Tracy (Recommended for This Issue)

Tracy is the best tool for understanding frame timing issues in game engines.

### Setup

```toml
# In Cargo.toml
[dependencies]
bevy = { version = "0.14", features = ["trace", "trace_tracy"] }
tracy-client = { version = "0.17", features = ["enable"] }

[profile.release]
debug = true  # Needed for symbols
```

### Usage

```rust
// Add to main.rs
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::log::LogPlugin;

app.add_plugins(DefaultPlugins.set(LogPlugin {
    level: bevy::log::Level::INFO,
    filter: "wgpu=error,naga=error".to_string(),
}))
```

Run with:
```bash
cargo build --release --features bevy/trace_tracy
./target/release/seaview --source-coordinates zup assets/test_sequences/right-hander

# In another terminal, run Tracy profiler
```

### What to Look For
- Frame time spikes during mesh loading
- GPU vs CPU bottlenecks
- Memory allocation patterns
- Asset loading timeline

## 2. Puffin (Easier Alternative)

Puffin is a simpler profiler that integrates well with Bevy.

### Setup

```toml
[dependencies]
puffin = "0.19"
puffin_egui = "0.28"
bevy_egui = "0.28"

[dependencies.bevy]
version = "0.14"
features = ["trace", "trace_chrome"]
```

### Integration

```rust
use puffin_egui::puffin;

fn setup_profiling(app: &mut App) {
    // Enable puffin
    puffin::set_scopes_on(true);
    
    app.add_plugins(bevy_egui::EguiPlugin)
       .add_systems(Update, profiler_ui);
}

fn profiler_ui(
    mut contexts: EguiContexts,
    mut puffin_ui: Local<bool>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Toggle with F1
    if keyboard.just_pressed(KeyCode::F1) {
        *puffin_ui = !*puffin_ui;
    }
    
    if *puffin_ui {
        puffin_egui::profiler_window(contexts.ctx_mut());
    }
}

// Profile specific functions
fn load_mesh_with_profiling() {
    puffin::profile_function!();
    
    // Your mesh loading code
    {
        puffin::profile_scope!("STL parsing");
        // Parse STL
    }
    
    {
        puffin::profile_scope!("Mesh creation");
        // Create mesh
    }
    
    {
        puffin::profile_scope!("GPU upload");
        // Upload to GPU
    }
}
```

## 3. Bevy Built-in Diagnostics (Already Added)

Enhance what you have:

```rust
use bevy::diagnostic::{
    DiagnosticsStore, 
    FrameTimeDiagnosticsPlugin,
    EntityCountDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
};

app.add_plugins((
    FrameTimeDiagnosticsPlugin,
    EntityCountDiagnosticsPlugin,
    SystemInformationDiagnosticsPlugin,
))
.add_systems(Update, log_diagnostics);

fn log_diagnostics(
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
) {
    static mut LAST_LOG: f32 = 0.0;
    
    let current = time.elapsed_seconds();
    unsafe {
        if current - LAST_LOG > 1.0 {
            LAST_LOG = current;
            
            // Log frame time percentiles
            if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(average) = fps.average() {
                    info!("FPS: {:.0} (min: {:.0}, max: {:.0})", 
                        average,
                        fps.history_mut().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0),
                        fps.history_mut().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0)
                    );
                }
            }
        }
    }
}
```

## 4. CPU Profiling with cargo-flamegraph

Best for identifying CPU bottlenecks:

```bash
# Install
cargo install flamegraph

# Profile
cargo flamegraph --release --bin seaview -- --source-coordinates zup assets/test_sequences/sphere-fluid

# On Linux, may need:
echo -1 | sudo tee /proc/sys/kernel/perf_event_paranoid
```

## 5. Memory Profiling

For memory issues (likely with 156M+ faces):

### Using heaptrack (Linux)
```bash
sudo apt install heaptrack
heaptrack ./target/release/seaview -- --source-coordinates zup assets/test_sequences/right-hander
heaptrack_gui heaptrack.seaview.*.gz
```

### Using dhat
```toml
[dependencies]
dhat = "0.3"

[features]
dhat-heap = []
```

```rust
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();
    
    // Your app
}
```

## 6. GPU Profiling

### RenderDoc
Best for GPU bottleneck analysis:

1. Install RenderDoc
2. Launch your app through RenderDoc
3. Capture frames during performance issues
4. Analyze draw calls, GPU timings, and memory usage

### wgpu Profiling
```rust
// Enable wgpu profiling
app.insert_resource(WgpuSettings {
    features: WgpuFeatures::TIMESTAMP_QUERY,
    ..default()
});
```

## Specific Areas to Profile for Your Issue

1. **Mesh Creation Pipeline**
   - STL parsing time
   - Vertex/index buffer creation
   - Normal calculation (especially with coordinate transformation)
   - GPU upload time

2. **Memory Allocation**
   - Large allocations during mesh loading
   - Memory fragmentation
   - GPU memory usage

3. **Frame Timing**
   - Identify which system causes frame spikes
   - Check if it's CPU or GPU bound
   - Look for synchronization points

4. **Asset Pipeline**
   - Time spent in `Assets<Mesh>::add()`
   - Handle creation overhead
   - Asset event processing

## Quick Performance Fixes to Try

Before deep profiling, try these:

1. **Limit concurrent loads**
```rust
const MAX_CONCURRENT_LOADS: usize = 2; // Instead of 8
```

2. **Add mesh LOD (Level of Detail)**
```rust
// Skip more faces for distant viewing
if total_faces > 1_000_000 {
    // Implement aggressive face skipping
}
```

3. **Pre-process meshes**
```bash
# Convert to more efficient format offline
cargo run --bin preprocess_stl -- input.stl output.mesh
```

4. **Use mesh instancing for repeated geometry**

5. **Implement streaming loading**
   - Load visible frames first
   - Load ahead based on playback direction

## Profiling Commands Summary

```bash
# Tracy
cargo build --release --features bevy/trace_tracy
./target/release/seaview --source-coordinates zup assets/test_sequences/right-hander

# Flamegraph
cargo flamegraph --release --bin seaview -- --source-coordinates zup assets/test_sequences/sphere-fluid

# Perf (Linux)
perf record --call-graph=dwarf ./target/release/seaview -- --source-coordinates zup assets/test_sequences/sphere-fluid
perf report

# Simple timing
time ./target/release/seaview -- --source-coordinates zup assets/test_sequences/sphere-fluid
```

## Next Steps

1. Start with Tracy to understand the frame timing issues
2. Use flamegraph to identify CPU bottlenecks in mesh processing
3. Check GPU memory usage with RenderDoc
4. Implement targeted optimizations based on findings