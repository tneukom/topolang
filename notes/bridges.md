# Wire that cross are hard to do, possible solutions

## Bridge
The most natural solution is that one wire passes under the other wire. We could
switch from a 2d world to a 3d world, however this would increase complexity
considerably:
- Boundaries become 2d interface surfaces
- An interface surface has a 1d boundary itself
- Would require some kind of data structure to store 3d world

## Patterns with optional parts
Could make it easier to match wires with 0, 1, 2, ... crossings in a single
rule.

## Highlight regions
If a highlight color appears in the "after" part of a rule, a new highlight
region will be created that is not part of the normal world. These regions are
always generated from the real world and ignored when saving to disk.

If a highlight region is a subset of another highlight region with the same
color it is deleted.

There is a single editable highlight region for drawing highlights in rules.

There can be distinct highlight regions of the same color that overlap. One
cannot be a subset of the other though.