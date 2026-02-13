# Settings & Environment System — Comprehensive Plan

> **Purpose**: Extend `seaview.toml` to persist all user-configurable state so
> that opening a sequence directory restores the exact working environment.
> **Cameras** are named bookmarks you save, switch between, and recall.
> The **environment** is the logical grouping of everything about the visual
> appearance: lighting, materials, imported props, background, post-processing.

---

## Assumptions

1. **One `seaview.toml` per sequence directory** — saves everything about this scene.
2. **Cameras are a named map** — `[cameras.<name>]`. `active_camera` selects which one is live. Scene-specific (depends on geometry), so they live at the top level, not inside environment.
3. **The environment lives inside `seaview.toml`** — not a separate file (yet). Future: export/import named environment presets.
4. **An environment bundles visual appearance only** — NOT cameras, playback, mesh bounds, or coordinates.
5. **Materials are a named map** — `[environment.materials.<name>]`. One is marked `active` for the primary mesh. Props reference materials by name.
6. **TOML remains the format** — human-editable, merge-friendly, `serde` already wired up.
7. **All sections are `Option<T>` with `#[serde(default)]`** — missing = use app default. Partial files always valid. Old files still parse.
8. **Save is explicit** (💾 button) — not auto-saved on slider drag.
9. **Load happens once at startup** — values populate Bevy resources; runtime state lives in `Res`/`ResMut`.
10. **Bevy `Color` stored as `[f32; 3]` sRGB arrays** — keeps TOML readable, avoids Bevy-version coupling.
11. **`NightLightingConfig` already derives `Serialize`/`Deserialize`**; `MaterialConfig` needs a serde bridge struct.
12. **Global defaults (`~/.config/seaview/defaults.toml`)** are a future layer — same struct, merged under per-directory values. Not in scope now.

---

## Architecture: What is an Environment?

An environment is "everything about the scene that isn't the simulation data or the camera viewpoint." It is the stage, the lighting, the materials, the props, the atmosphere.

```text
seaview.toml
│
├── active_camera = "default"         — which camera is live
├── [cameras.default]                 — named viewpoint: position, rotation
├── [cameras.closeup]                 — another saved viewpoint
├── [cameras.overhead]                — etc.
│
├── [sequence]                        — source coordinate system (data-specific)
├── [playback]                        — speed, loop (workflow)
├── [mesh]                            — cached AABB (data-derived)
│
└── [environment]                     — THE VISUAL LOOK ← all new
    ├── [environment.lighting]        — spot rig + global light toggle
    ├── [environment.materials.fluid] — named material for the fluid mesh
    ├── [environment.materials.ground]— named material for a ground plane (example)
    ├── [environment.props]           — imported reference objects (future)
    ├── [environment.background]      — clear color, skybox (future)
    └── [environment.post_processing] — bloom, tone mapping, SSR (future)
```

### What belongs where

| ✅ Environment (the look) | ✅ Cameras (scene-specific, named) | ❌ Neither (data/workflow) |
|---|---|---|
| Spot light count, layout, height, cone, intensity, color, range, coverage | Named viewpoints (position + rotation) | Playback speed / loop |
| Global lighting on/off | Active camera selector | Mesh bounds (computed from data) |
| Named materials: color, roughness, metallic, reflectance, emissive, alpha | *(future)* FOV, near/far clip per camera | Source coordinates |
| Marker visibility & size | | Sequence file patterns |
| Imported props / reference geometry | | |
| Background color / skybox | | |
| Post-processing: bloom, tone mapping, AO, SSR | | |
| Ground plane / grid | | |
| Fog / atmosphere | | |

---

## How It Works Today

### Settings struct (`lib/settings.rs`)

```text
Settings
├── camera:   Option<CameraSettings>      ← SINGLE position [x,y,z] + rotation [pitch,yaw,roll]
├── sequence: Option<SequenceSettings>     ← source_coordinates string
├── playback: Option<PlaybackSettings>     ← speed, loop
└── mesh:     Option<MeshBoundsSettings>   ← cached AABB (min/max/dimensions)
```

Only one camera. No environment. No named anything.

### Save path

1. User clicks 💾 → `SaveViewEvent` fired.
2. `handle_save_view` in `main.rs` reads camera `Transform`, `UiState.playback`, `SourceOrientation`.
3. Populates `settings_res.settings.camera`, `.playback`, `.sequence`.
4. Calls `settings_res.save()` → merges into existing TOML on disk.

### Load path

1. `main()` calls `Settings::load_from_dir()` before app build.
2. `setup()` reads `SettingsResource`:
   - Camera → spawn transform
   - Mesh bounds → insert `MeshDimensions`
   - Playback → mutate `UiState`
3. Sequence/coordinates applied via CLI + settings merge.

### What is NOT saved today

| Config area | Runtime resource | Saved? |
|---|---|---|
| Camera | single `CameraSettings` | ✅ (but only one, no naming) |
| Lighting rig | `NightLightingConfig` | ❌ |
| Material | `MaterialConfig` (single) | ❌ |
| Multiple cameras | — | ❌ |
| Global ambient light | hard-coded in `setup()` | ❌ |
| Background | hard-coded clear color | ❌ |
| Post-processing | hard-coded SSR, no bloom | ❌ |

---

## What Needs to Change

### 1. Cameras → named map

`CameraSettings` already exists. Changes:

- `camera: Option<CameraSettings>` → `cameras: HashMap<String, CameraSettings>` + `active_camera: Option<String>`.
- On save: current camera transform saved under active name (default: `"default"`).
- On load: restore the camera named by `active_camera`. All others stored for switching.
- **Migration**: if old `[camera]` section found (no `[cameras.*]`), auto-migrate to `cameras.default`.

### 2. `LightingSettings` — TOML bridge for `NightLightingConfig`

`NightLightingConfig` already derives `Serialize`/`Deserialize` but its `Color`
field needs the `[f32; 3]` array treatment for clean TOML. Add a bridge struct:

```text
LightingSettings {
    num_lights:              usize,
    height:                  f32,
    cone_angle:              f32,
    placement_algorithm:     String,     // "uniform_grid" | "hexagonal_packing" | ...
    enabled:                 bool,
    global_lighting_enabled: bool,
    intensity:               f32,
    color:                   [f32; 3],
    range:                   f32,
    show_markers:            bool,
    marker_size:             f32,
    coverage_pct:            f32,
}
```

With `From<&NightLightingConfig>` and `Into<NightLightingConfig>`.

### 3. `MaterialSettings` — TOML bridge for `MaterialConfig`

`MaterialConfig` uses Bevy `Color` (not serde-friendly for TOML). Add:

```text
MaterialSettings {
    base_color:            [f32; 3],
    perceptual_roughness:  f32,
    metallic:              f32,
    reflectance:           f32,
    emissive:              [f32; 3],
    emissive_intensity:    f32,
    double_sided:          bool,
    alpha_mode:            String,       // "opaque" | "mask" | "blend"
    alpha_cutoff:          f32,
}
```

With `From<&MaterialConfig>` and `Into<MaterialConfig>`.

### 4. `EnvironmentSettings` — the grouping struct

```text
EnvironmentSettings {
    lighting:        Option<LightingSettings>,
    materials:       HashMap<String, MaterialSettings>,   // named map; "fluid" is the default key
    // --- future sections, all Option so they're omitted from TOML when absent ---
    props:           Option<Vec<PropSettings>>,
    background:      Option<BackgroundSettings>,
    post_processing: Option<PostProcessingSettings>,
}
```

Future section sketches (not implemented now, just reserved in the struct):

```text
PropSettings {                          // an imported reference object
    name:      String,
    file_path: String,                  // relative to sequence dir
    position:  [f32; 3],
    rotation:  [f32; 3],
    scale:     [f32; 3],
    material:  Option<String>,          // key into materials map
    visible:   bool,
}

BackgroundSettings {
    color:     Option<[f32; 3]>,        // solid clear color
    skybox:    Option<String>,          // path to HDRI / cubemap
}

PostProcessingSettings {
    bloom_enabled:    bool,
    bloom_intensity:  f32,
    tone_mapping:     String,           // "none" | "reinhard" | "aces" | ...
    exposure:         f32,
    ssr_enabled:      bool,
    ao_enabled:       bool,
}
```

### 5. Expand `Settings` top-level struct

```text
Settings
├── active_camera: Option<String>               ← NEW (default: "default")
├── cameras:       HashMap<String, CameraSettings>  ← replaces camera: Option<CameraSettings>
├── camera:        Option<CameraSettings>       (legacy, auto-migrated on load)
├── sequence:      Option<SequenceSettings>     (existing)
├── playback:      Option<PlaybackSettings>     (existing)
├── mesh:          Option<MeshBoundsSettings>   (existing)
└── environment:   Option<EnvironmentSettings>  ← NEW
```

Migration in `Settings::load_from_dir()`: if `camera` is `Some` and `cameras` is empty,
move it into `cameras["default"]` and set `active_camera = "default"`.

### 6. Expand `handle_save_view` (save path)

Add `Res<NightLightingConfig>` and `Res<MaterialConfig>` to system params.
Save current camera under `active_camera` name (or `"default"`).
Convert lighting/material to settings structs, populate `settings.environment`,
call `save()`.

### 7. Expand `setup()` (load path)

After playback restore:

- Look up `active_camera` in `cameras` map → apply transform (fall back to first entry, then hardcoded default).
- Store full cameras map in a new `SavedCameras` resource for runtime switching.
- If `environment.lighting` present → build `NightLightingConfig` → `commands.insert_resource()`.
- If `environment.materials["fluid"]` present → build `MaterialConfig` → `commands.insert_resource()`.

Plugin `build()` runs during app construction (inserts defaults). `Startup` systems
run after, so `commands.insert_resource()` overwrites the defaults cleanly.

---

## Multiple Materials — Design Note

Today there is one `MaterialConfig` resource applied to the single sequence mesh.
The named materials map (`HashMap<String, MaterialSettings>`) prepares for:

- **Multiple mesh parts** — if meshes gain material IDs or vertex groups, each can reference a material by name.
- **Props** — imported reference objects reference materials by name.
- **Quick switching** — UI dropdown to swap the fluid mesh between "fluid", "wireframe", "glass", etc.

For now, the convention is:
- `"fluid"` — the default key, applied to the sequence mesh.
- Any other keys are stored but not applied until props or multi-material support lands.

At runtime, `MaterialConfig` stays as the single active resource. The named map
is a persistence/preset concept only.

---

## Implementation Plan

### Phase 1 — Serde bridge structs + Settings expansion

**Files:** `lib/settings.rs`

1. Add `LightingSettings` with `#[derive(Serialize, Deserialize, Default)]` and `#[serde(default)]` on all fields.
2. Add `MaterialSettings` with same.
3. Add `EnvironmentSettings` with `lighting: Option<LightingSettings>`, `materials: HashMap<String, MaterialSettings>`. Future fields as `Option` (not implemented yet, just reserved).
4. Implement `From` conversions: `&NightLightingConfig ↔ LightingSettings`, `&MaterialConfig ↔ MaterialSettings`.
5. Expand `Settings`: add `active_camera: Option<String>`, `cameras: HashMap<String, CameraSettings>`, `environment: Option<EnvironmentSettings>`.
6. Add migration logic: old `[camera]` → `cameras["default"]`.
7. Add helpers: `Settings::set_environment(...)`, `Settings::save_camera(name, transform)`.
8. Unit tests: round-trip, partial TOML, backward compat, camera migration.

### Phase 2 — Save path

**Files:** `main.rs` (`handle_save_view`)

1. Add `Res<NightLightingConfig>` and `Res<MaterialConfig>` to system params.
2. Save camera transform under `active_camera` name (default `"default"`).
3. Build `EnvironmentSettings` from runtime resources.
4. Set `settings_res.settings.environment` before `save()`.
5. Verify: click 💾 → `seaview.toml` contains `[cameras.default]`, `[environment.lighting]`, `[environment.materials.fluid]`.

### Phase 3 — Load path

**Files:** `main.rs` (`setup` or new startup system)

1. Run migration (old `[camera]` → `cameras["default"]`).
2. Look up `active_camera` in cameras map → apply transform to spawned camera.
3. Store cameras map in `SavedCameras` resource (for future UI switching).
4. If `environment.lighting` present → convert to `NightLightingConfig` → `commands.insert_resource()`.
5. If `environment.materials["fluid"]` present → convert to `MaterialConfig` → `commands.insert_resource()`.
6. Verify: manually edit `seaview.toml` → launch → values apply.

### Phase 4 — Camera switching UI (lightweight)

**Files:** `app/ui/systems/playback_controls.rs` or new camera panel

1. Add `SavedCameras` resource: `HashMap<String, CameraSettings>` + `active: String`.
2. Show dropdown/list of saved cameras near the 💾 button.
3. Click a name → apply that camera's transform.
4. "Save as..." → prompt for name, save current transform to map.
5. 💾 button persists the full map to TOML as before.

### Phase 5 — Tests & edge cases

1. Old `seaview.toml` with `[camera]` (no `[cameras.*]`) auto-migrates on load.
2. Old `seaview.toml` without `[environment]` loads fine (defaults used).
3. Partial `[environment.lighting]` (e.g. only `num_lights`) fills rest from defaults.
4. Full save → quit → relaunch round-trips all values including all named cameras.
5. Materials map with unknown keys is preserved on merge (no data loss).

---

## TOML Example (target state)

```toml
active_camera = "default"

[cameras.default]
position = [120.5, 85.3, 200.0]
rotation = [-15.0, 45.0, 0.0]

[cameras.closeup]
position = [10.0, 5.0, 15.0]
rotation = [-30.0, 90.0, 0.0]

[cameras.overhead]
position = [50.0, 300.0, 50.0]
rotation = [-89.0, 0.0, 0.0]

[sequence]
source_coordinates = "zup"

[playback]
speed = 1.5
loop = true

[mesh]
min = [0.0, 0.0, 0.0]
max = [75.0, 50.0, 75.0]
dimensions = [75.0, 50.0, 75.0]

[environment.lighting]
num_lights = 16
height = 80.0
cone_angle = 45.0
placement_algorithm = "hexagonal_packing"
enabled = true
global_lighting_enabled = false
intensity = 1200000.0
color = [1.0, 0.95, 0.85]
range = 600.0
show_markers = false
marker_size = 0.5
coverage_pct = 150.0

[environment.materials.fluid]
base_color = [0.3, 0.5, 0.8]
perceptual_roughness = 0.4
metallic = 0.1
reflectance = 0.5
emissive = [0.0, 0.0, 0.0]
emissive_intensity = 0.0
double_sided = true
alpha_mode = "opaque"
alpha_cutoff = 0.5

# --- future sections (not yet implemented) ---

# [environment.materials.ground]
# base_color = [0.4, 0.35, 0.3]
# perceptual_roughness = 0.9
# metallic = 0.0
# ...

# [environment.background]
# color = [0.05, 0.05, 0.1]

# [environment.post_processing]
# bloom_enabled = true
# bloom_intensity = 0.3
# tone_mapping = "aces"
# ssr_enabled = true
```

---

## Future Extensions (design accommodates, not in scope now)

| Feature | How it fits |
|---|---|
| **Named environment presets** | Export `[environment]` section to standalone `.toml`; import overwrites `[environment]` |
| **Global user defaults** | `~/.config/seaview/defaults.toml` — same `Settings` struct, merged under per-directory values |
| **Camera animations / paths** | `[cameras.<name>.keyframes]` — fly-through paths between named cameras |
| **Camera properties** | FOV, near/far clip, depth of field per named camera |
| **Props / reference objects** | `[[environment.props]]` array of tables — file path + transform + material ref |
| **Background / skybox** | `[environment.background]` — color or HDRI path |
| **Post-processing** | `[environment.post_processing]` — bloom, tone mapping, exposure, SSR, AO |
| **Ground plane / grid** | `[environment.ground]` — size, divisions, color, visible |
| **Fog / atmosphere** | `[environment.atmosphere]` — fog color, density, start/end distance |
| **Multi-material assignment** | Map mesh vertex groups / regions to material keys |

---

## Risk & Decision Notes

- **`Color` serde**: Stored as `[f32; 3]` sRGB arrays. Intentional — keeps TOML readable, avoids Bevy-version coupling.
- **`PlacementAlgorithm` as string**: Currently the enum derives `Serialize`/`Deserialize` producing `"UniformGrid"`. The bridge struct stores a `String` and matches manually, letting us decouple TOML naming from Rust enum variants. Use `snake_case` in TOML.
- **Camera migration**: Old `[camera]` is a single struct. New `[cameras.<name>]` is a map. `load_from_dir()` checks for the old format and migrates transparently. The old `[camera]` key is removed on next save.
- **Resource insertion order**: `NightLightingPlugin` and `MaterialConfigPlugin` insert defaults in `build()`. `Startup` systems run after, so `commands.insert_resource()` cleanly overwrites.
- **`#[serde(default)]` everywhere**: Every field in every settings struct gets a sensible default. This means we can add fields in any release without breaking existing files.
- **HashMap for materials**: TOML's `[environment.materials.<name>]` maps naturally to `HashMap<String, MaterialSettings>`. Unknown keys are preserved on save-merge.