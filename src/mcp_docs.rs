//! Long-form MCP copy. MCP is the primary interface; keep thorough and free of host paths.

pub const SERVER_INSTRUCTIONS: &str = r#"stl2ascii — ASCII STL viewer and mesh QA over local stdio MCP.

PURPOSE
  Numerical mesh inspection for CAD/converter workflows, plus orthographic ASCII for confirmation. Prefer measurement tools for decisions; use render tools for visual aha. Paths are local filesystem paths. Units match the STL.

WHICH TOOL
  • mesh_stats — AABB, area, volume, topology. Optional path_b returns {a,b,clip?} side-by-side.
  • mesh_diff — distance histograms + outside volumes over volume_bounds (clip box when clip set).
  • volume_in_box — mesh ∩ box volume; read estimated_error (= voxel_size³) to pick grid (max 64).
  • render_stl — single-mesh ASCII. clip is a HARD geometric cut; camera auto-fits the clip unless focus/radius set. Status echoes clip.
  • render_overlay — A vs B in one frame: #=A only, %=B only, *=both.
  • render_deviation — shade display mesh by distance buckets to path_ref (density ramp).
  • list_presets / list_shade_modes — discovery.

CLIP
  clip/box = [x,y,z,dx,dy,dz] min+size. Hard-cuts triangles to the AABB (not whole-triangle filter). mesh_diff volumes integrate only inside volume_bounds (= clip when provided). For tiny crops, volume_in_box is often clearer than mesh_diff volumes.

CAMERA
  zoom>1 closer about orbit target; pan_x/pan_y = fraction of fit_extent; focus/radius override defaults. With clip and no focus/radius, target=clip center and fit≈0.65×max(dx,dy,dz).

APPROXIMATIONS
  Voxel occupancy + raycast parity are approximate. Open/non-manifold meshes make inside tests unreliable. surface_area_in_box is centroid-based — not comparable across tessellation densities.

WORKFLOWS
  1) mesh_stats (optionally with path_b).
  2) mesh_diff; use clip for localized volume_bounds.
  3) volume_in_box slabs; choose grid via estimated_error.
  4) render_overlay / render_deviation / render_stl for confirmation; read clip on status line.
"#;

pub const RENDER_STL: &str = "Render one STL to bordered orthographic ASCII plus a status line (camera, stats, view AABB, clip if set). Optional clip HARD-CUTS triangles to [x,y,z,dx,dy,dz] and auto-fits the camera to that box unless focus/radius are set. Use for visual confirmation after measurements.";

pub const RENDER_OVERLAY: &str = "Render two STLs in one ASCII frame with shared camera/clip. Glyphs: # = A only, % = B only, * = both, space = empty. Best single-image answer for original-vs-converted comparison. Same clip/camera semantics as render_stl (hard cut + auto-fit).";

pub const RENDER_DEVIATION: &str = "Rasterize path, shade each filled cell by unsigned distance to path_ref using density ramp mapped to buckets <=0.01, <=0.05, <=0.1, <=0.5, <=1, >1. Replaces guessing from a boolean difference solid. Optional clip hard-cuts both meshes and auto-fits camera.";

pub const MESH_STATS: &str = "Load STL stats: triangles, AABB, area, signed volume, topology (boundary_edges, non_manifold_edges, components, is_closed, genus). Optional clip hard-cuts before stats. Optional path_b: return {a, b, clip?} JSON instead of a flat single-mesh object (single-path responses stay flat MeshStats for compatibility).";

pub const MESH_DIFF: &str = "Compare two STLs: unsigned distance histograms both ways + volume_a_outside_b / volume_b_outside_a over volume_bounds. When clip is set, meshes are hard-cut and volume_bounds equals the clip box (volumes honour the crop). Prefer volume_in_box for very small slabs. Results include method/notes/voxel_size/grid_dims/volume_bounds.";

pub const VOLUME_IN_BOX: &str = "Volume of mesh ∩ box via voxel occupancy, plus surface_area_in_box (centroid-in-box areas — NOT comparable across tessellation densities). estimated_error = voxel_size³ (discretization scale). grid default 64, clamped 8…64. See grid_requested vs grid_used.";

pub const LIST_PRESETS: &str = "List camera presets with yaw/pitch in degrees: front, back, left, right, top, bottom, iso.";

pub const LIST_SHADE_MODES: &str = "List ASCII shade modes: density ( .:-=+*#%@ ), lines (line glyphs only).";
