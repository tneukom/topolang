*Region*: An area of constant color

*SeamAtom*: Maximal connected series of sides between two Regions. Meaning
constant color on the left and the right side. 

*Seam*: Connected series of sides with constant left side.

*SeamChain*: Connected list of SeamAtoms

Given an area its *Boundary* is the set of sides where left(side) is contained 
in the area and right(side) is not.
The left side of the boundary is the original area and the right side is the 
complement of the area.
