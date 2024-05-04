**Seams**
- Border that contains a Seam
- All Seams contained in a Border
- Left/Right color of a side

**Regions**
- All Borders of a Region
- Blit a Region to a Pixmap
- 

**Modifying**
- All Regions that intersect a set of pixels


Operations
==========
It's easy to accidentally invalidate seams while changing the topology.


- Copy/Clear right side of a boundary is easy to do if 
- Setting the color of a Region is easy if the new color is different
  from all current neighboring colors.
