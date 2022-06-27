# Subgroups of affine transformations on Q^2

Similarity: Uniform scaling, translation, mirror, rotation
https://en.wikipedia.org/wiki/Similarity_(geometry)
https://en.wikipedia.org/wiki/Conformal_linear_transformation
https://en.wikipedia.org/wiki/Conformal_map

Isometry (or Rigid Transformation): No scaling, translation, rotation
https://en.wikipedia.org/wiki/Isometry
https://en.wikipedia.org/wiki/Rigid_transformation

- `Isometry<i64>` is invertible if rotation is restricted to multiples of 90°

## Similarities

### Angle preserving <Au, Av> = k^2<u, v> for some k

**Claim:** Equivalent to A*A^T = k^2 Id

<=: <Au, Av> = u^T * A^T * A * v = u^T * k^2 Id * v = k^2<u, v>

=>: Ae_i is the i-th column of A, follows from <Ae_i, Ae_j> = k^2 <e_i, e_j>

**Claim:** Rotation with uniform scale is commutative.

**Uniform scaling, translations, rotations**: For R^2 we can represent 
A as A = k * R * T where R is a rotation and T is a translation.
For Q^2 this does not work because k might not be in Q.

### Full linear group GL(2) over Q
If A = [[a, b], [c, d]]
then A^-1 = 1/(ad - bc) * [[d, -b], [-c, a]]

Not commutative

Add new classes QMatrix, QAffine!

### Resize maps
Non-uniform scaling and translations. Maps one rectangle to another. What would
be a good name for this type of transformation?


### TODO
- Rename QComplex to QPoint
- Rename QiAffine to QSimilarity
- Rename QiRect, QiHalfSpace, QiLineSegment, ...
