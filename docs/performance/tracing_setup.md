# Chrome Tracing Format Setup for Performance Analysis

This guide shows how to use Chrome's tracing format for easier programmatic analysis of performance data in Seaview.

## Chrome Tracing Format Benefits

- JSON format that's easy to parse
- Can be viewed in Chrome's chrome://tracing
- Can be analyzed with Python/scripts
- Works with Perfetto UI (ui.perfetto.dev)
- Captures full timeline data

## Setup

### 1. Enable Tracing in Cargo.toml

```toml
[dependencies]
bevy = { version = "0.14", features = ["trace", "trace_chrome"] }
tracing-chrome = "0.7"

[profile.release]
debug = true  # Keep symbols for better traces
```

### 2. Initialize Chrome Tracing

```rust
use tracing_chrome::{ChromeLayerBuilder, FlushGuard};
use tracing_subscriber::{prelude::*, registry::Registry};

static mut FLUSH_GUARD: Option<FlushGuard> = None;

fn setup_chrome_tracing() {
    let (chrome_layer, guard) = ChromeLayerBuilder::new()
        .file("./trace-output.json")
        .include_args(true)
        .include_locations(true)
        .build();
    
    // Store guard to ensure flush on exit
    unsafe {
        FLUSH_GUARD = Some(guard);
    }
    
    let subscriber = Registry::default().with(chrome_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn main() {
    setup_chrome_tracing();
    
    // Your Bevy app...
    App::new()
        .add_plugins(DefaultPlugins)
        // ...
        .run();
}
```

### 3. Add Tracing Spans to Key Functions

```rust
use tracing::{info_span, instrument};

// Instrument the parallel loader
#[instrument(skip_all, fields(path = %path.display()))]
pub fn load_stl_parallel(
    path: &Path,
    use_fallback: bool,
) -> Result<(Mesh, MeshLoadStats), Box<dyn Error>> {
    let _span = info_span!("load_stl_parallel").entered();
    
    // Parse STL
    let parse_span = info_span!("parse_stl").entered();
    let stl = parse_stl(path)?;
    drop(parse_span);
    
    // Process faces
    let process_span = info_span!("process_faces", face_count = stl.faces.len()).entered();
    let mesh = process_stl_faces(&stl)?;
    drop(process_span);
    
    // Create GPU buffers
    let gpu_span = info_span!("create_gpu_buffers").entered();
    let final_mesh = create_bevy_mesh(mesh)?;
    drop(gpu_span);
    
    Ok((final_mesh, stats))
}

// Instrument the cache update
#[instrument(skip_all)]
pub fn update_cache_from_loads(
    loader: Res<AsyncStlLoader>,
    mut cache: ResMut<AsyncMeshCache>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let _span = info_span!("update_cache").entered();
    
    let process_span = info_span!("process_completed_loads").entered();
    loader.process_completed_loads(&mut meshes);
    drop(process_span);
    
    let cache_span = info_span!("update_cache_entries").entered();
    // ... cache update logic
    drop(cache_span);
}

// Instrument the main render loop
#[instrument(skip_all)]
fn update_mesh_visibility(
    mut commands: Commands,
    sequence_manager: Res<SequenceManager>,
    mut cache: ResMut<AsyncMeshCache>,
) {
    let _span = info_span!("update_mesh_visibility", frame = sequence_manager.current_frame).entered();
    // ... visibility logic
}
```

## Viewing Traces

### Chrome DevTools
1. Open Chrome and navigate to `chrome://tracing`
2. Click "Load" and select `trace-output.json`
3. Use WASD to navigate, scroll to zoom

### Perfetto (Recommended)
1. Go to https://ui.perfetto.dev
2. Click "Open trace file"
3. Select `trace-output.json`

## Analyzing Traces Programmatically

### Python Script for Analysis

```python
import json
import pandas as pd
from collections import defaultdict

def analyze_trace(filename):
    with open(filename, 'r') as f:
        events = json.load(f)
    
    # Group events by name
    durations = defaultdict(list)
    
    for event in events:
        if event['ph'] == 'X':  # Complete event
            name = event['name']
            duration = event['dur'] / 1000.0  # Convert to ms
            durations[name].append(duration)
    
    # Calculate statistics
    stats = {}
    for name, times in durations.items():
        stats[name] = {
            'count': len(times),
            'total_ms': sum(times),
            'avg_ms': sum(times) / len(times),
            'max_ms': max(times),
            'min_ms': min(times)
        }
    
    # Convert to DataFrame for easy analysis
    df = pd.DataFrame.from_dict(stats, orient='index')
    df = df.sort_values('total_ms', ascending=False)
    
    print("Top 10 time-consuming operations:")
    print(df.head(10))
    
    # Find frame time spikes
    frame_events = [e for e in events if e['name'] == 'frame' and e['ph'] == 'X']
    frame_times = [e['dur'] / 1000.0 for e in frame_events]
    
    spikes = [(i, t) for i, t in enumerate(frame_times) if t > 33.33]  # > 30 FPS
    print(f"\nFound {len(spikes)} frame spikes (>33ms)")
    
    return df, frame_times

# Usage
df, frame_times = analyze_trace('trace-output.json')
```

### Find Specific Bottlenecks

```python
def find_mesh_loading_bottlenecks(filename):
    with open(filename, 'r') as f:
        events = json.load(f)
    
    mesh_events = [e for e in events if 'mesh' in e['name'].lower() or 'stl' in e['name'].lower()]
    
    # Group by operation type
    operations = defaultdict(lambda: {'count': 0, 'total_time': 0})
    
    for event in mesh_events:
        if event['ph'] == 'X':
            op_type = event['name']
            operations[op_type]['count'] += 1
            operations[op_type]['total_time'] += event['dur'] / 1000.0
    
    # Print summary
    for op, stats in sorted(operations.items(), key=lambda x: x[1]['total_time'], reverse=True):
        avg_time = stats['total_time'] / stats['count'] if stats['count'] > 0 else 0
        print(f"{op}: {stats['count']} calls, {stats['total_time']:.2f}ms total, {avg_time:.2f}ms avg")
```

## Key Metrics to Track

1. **Frame Time Distribution**
   - Average, P95, P99 frame times
   - Frame time spikes > 16.67ms (60 FPS target)

2. **Mesh Loading Performance**
   - STL parsing time per MB
   - Vertex processing time per 1M vertices
   - GPU upload time
   - Total time from request to ready

3. **Memory Allocation Patterns**
   - Large allocations during loading
   - Allocation frequency
   - Peak memory usage

4. **System Bottlenecks**
   - Which systems take longest
   - Parallel vs sequential execution
   - GPU vs CPU bound operations

## Integration with CI/CD

```yaml
# .github/workflows/performance.yml
- name: Run Performance Test
  run: |
    cargo build --release --features trace_chrome
    timeout 60s ./target/release/seaview --source-coordinates zup assets/test_sequences/sphere-fluid
    
- name: Analyze Performance
  run: |
    python scripts/analyze_trace.py trace-output.json > performance_report.txt
    
- name: Check Performance Regression
  run: |
    python scripts/check_regression.py trace-output.json --baseline baseline.json
```

## Tips for Effective Tracing

1. **Use Consistent Span Names**: Makes analysis easier
2. **Add Relevant Fields**: Include sizes, counts, paths
3. **Limit Trace Scope**: Don't trace every function
4. **Focus on Hot Paths**: Trace operations that run frequently
5. **Include Async Operations**: Trace both request and completion

## Example Output Format

The Chrome trace format outputs JSON like:
```json
[
  {
    "name": "load_stl_parallel",
    "cat": "function",
    "ph": "X",
    "ts": 1234567890,
    "dur": 45678,
    "pid": 1234,
    "tid": 5678,
    "args": {
      "path": "surface_000100.stl"
    }
  }
]
```

This format is easy to parse and analyze with standard tools.