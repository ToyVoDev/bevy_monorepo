# Future Spec Backlog

Items that need a brainstorm + spec before implementation. Roughly ordered by dependency.

---

## LOD / Rendering

### Imposter Heightmap Renderer (2–5km)
Terrain beyond 2km rendered as heightmap quads instead of voxel meshes. Deferred from the LOD system spec. Depends on: LOD system being live.
- How heightmaps are generated (same worldgen sampled at low res, or separate pass?)
- Normal mapping for visual depth
- Blending/transition at the 2km boundary with LOD2 super-chunks

### LOD Seam Stitching
Currently LOD ring transitions are hard swaps — visible pop when player crosses a ring boundary. Future work to smooth this (geometry stitching, cross-fade, or distance fog to hide it). Low priority until LOD system is live and the pop is measurable.

---

## Visuals

### Texture Atlas System
Replace per-voxel vertex colors with a texture atlas. Each voxel type gets a face texture. Depends on: understanding UV unwrapping strategy for greedy-merged quads (merged quads need either tiled UVs or separate meshing pass).

---

## Gameplay

### Inventory UI — Slot Icons and Counts
Hotbar slots currently show only a colored border for the active slot. Needs item icons and stack count display. Depends on: asset pipeline decision (embedded sprites vs loaded images).

### World Persistence (Save/Load)
Serialize `ChunkedWorld` to disk so player edits survive restarts. Needs a decision on format (flat binary per-chunk files, sqlite, etc.) and a chunk dirty-tracking strategy for partial saves.

### Crafting System
No design yet. Depends on: inventory system being stable.
