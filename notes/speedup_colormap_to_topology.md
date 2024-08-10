# Speedup Topology::new(color_map: ColorMap)

- Precompute borders per Tile (only interior sides) where each Border is
  a `Vec<Side>` or possible `SmallVec<Side>`. Combine partial borders using
  the sides between Tiles.
- Cache compiled Rules
- Compute Regions from Borders
  - Compute nearest Border to the left for each pixel tile by tile
  - Group Borders using the nearest Border map
  - Compute region map (if needed)

