//! Long-form MCP copy. MCP is the primary interface; keep thorough and free of host paths.

pub const SERVER_INSTRUCTIONS: &str = r#"stl2ascii — ASCII STL viewer and mesh QA over local stdio MCP.

PURPOSE
  Numerical mesh inspection for CAD/converter workflows, plus orthographic ASCII for confirmation. Prefer measurement tools for decisions; use render tools for visual aha. Paths are local filesystem paths. Units match the STL.

WHICH TOOL
  • mesh_stats — AABB, area, volume, topology, genus_notes, components_detail (top 5 by |volume|). Optional path_b → {a,b,clip?}.
  • mesh_diff — distance histograms + volume_extra (B∖A) / volume_missed (A∖B) over volume_bounds; clip_applied when clip set.
  • volume_in_box — mesh ∩ box volume; read estimated_error (= voxel_size³) to pick grid (max 64).
  • is_inside — raycast-parity point-in-mesh test (optional hard clip first).
  • slice — plane ∩ mesh as line segments + compact ASCII section preview (optional hard clip).
  • render_stl — single-mesh ASCII. clip is a HARD geometric cut (verts clamped into box); camera auto-fits unless focus/radius set. Status keeps clip: when truncating.
  • render_overlay — A vs B: #=A only, %=B only, *=both.
  • render_deviation — shade by distance buckets; status legend .≤0.01 :≤0.05 -≤0.1 =≤0.5 +≤1 #%>1.
  • list_presets / list_shade_modes — discovery.

CLIP
  clip/box = [x,y,z,dx,dy,dz] min+size. Hard-cuts triangles to the AABB (Sutherland–Hodgman + clamp into box; not a whole-triangle filter). mesh_stats AABB after clip ≈ clip size. mesh_diff volumes integrate only inside volume_bounds (= clip when clip_applied). For tiny crops, volume_in_box is often clearer.

CAMERA
  zoom>1 closer about orbit target; pan_x/pan_y = fraction of fit_extent; focus/radius override defaults. With clip and no focus/radius, target=clip center and fit≈0.65×max(dx,dy,dz).

APPROXIMATIONS
  Voxel occupancy + raycast parity are approximate. Open/non-manifold meshes make inside tests unreliable. surface_area_in_box is centroid-based — not comparable across tessellation densities. genus is null unless closed single-component manifold (see genus_notes).

WORKFLOWS
  1) mesh_stats (optionally with path_b); read components_detail / genus_notes.
  2) mesh_diff; use clip for localized volume_bounds; read volume_extra / volume_missed / clip_applied.
  3) volume_in_box slabs; choose grid via estimated_error.
  4) is_inside / slice for point and section probes.
  5) render_overlay / render_deviation / render_stl for confirmation; read clip on status line.
"#;

pub const RENDER_STL: &str = "Render one STL to bordered orthographic ASCII plus a status line (camera, stats, view AABB, clip if set). Optional clip HARD-CUTS triangles to [x,y,z,dx,dy,dz] (Sutherland–Hodgman; vertices clamped into the box) and auto-fits the camera to that box unless focus/radius are set. Status prefers keeping clip:[…] when truncating. Use for visual confirmation after measurements.";

pub const RENDER_OVERLAY: &str = "Render two STLs in one ASCII frame with shared camera/clip. Glyphs: # = A only, % = B only, * = both, space = empty. Best single-image answer for original-vs-converted comparison. Same clip/camera semantics as render_stl (hard cut + auto-fit).";

pub const RENDER_DEVIATION: &str = "Rasterize path, shade each filled cell by unsigned distance to path_ref. Status legend: .≤0.01 :≤0.05 -≤0.1 =≤0.5 +≤1 #%>1. Optional clip hard-cuts both meshes and auto-fits camera.";

pub const MESH_STATS: &str = "Load STL stats: triangles, AABB, area, signed volume, topology (boundary_edges, non_manifold_edges, components, is_closed, genus), genus_notes (always explains null genus), and components_detail (top 5 components by |volume| with per-component AABB/volume/center). Optional clip hard-cuts before stats (components re-welded by quantized positions). Optional path_b: return {a, b, clip?} JSON instead of a flat single-mesh object.";

pub const MESH_DIFF: &str = "Compare two STLs: unsigned distance histograms both ways + volume_a_outside_b / volume_b_outside_a (aliases volume_missed=A∖B, volume_extra=B∖A) over volume_bounds. When clip is set: both meshes hard-cut, clip_applied=true, volume_bounds equals the clip box. Prefer volume_in_box for very small slabs. Results include method/notes/voxel_size/grid_dims/volume_bounds/clip_applied.";

pub const VOLUME_IN_BOX: &str = "Volume of mesh ∩ box via voxel occupancy, plus surface_area_in_box (centroid-in-box areas — NOT comparable across tessellation densities). estimated_error = voxel_size³ (discretization scale). grid default 64, clamped 8…64. See grid_requested vs grid_used.";

pub const IS_INSIDE: &str = "Test whether point [x,y,z] is inside the mesh via raycast parity (+X). Optional clip hard-cuts first. Returns {inside, method:\"raycast_parity\", point, notes}. Approximate; open/non-manifold meshes are unreliable.";

pub const SLICE: &str = "Intersect mesh with plane axis=x|y|z at value. Returns JSON {axis,value,segment_count,segments:[[p0,p1],…],truncated,ascii_preview,notes}. Segments capped; ascii_preview is a compact ortho plot in the two free axes. Optional clip hard-cuts first.";

pub const LIST_PRESETS: &str = "List camera presets with yaw/pitch in degrees: front, back, left, right, top, bottom, iso.";

pub const LIST_SHADE_MODES: &str = "List ASCII shade modes: density ( .:-=+*#%@ ), lines (line glyphs only).";
