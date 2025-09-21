TopoLang
========

![Train Animation](readme_resources/train.gif)

TopoLang is an experimental programming "language" based on topological pattern
matching. In the example above there is a single rule, the left side is matched
and replaced by the right side.

TODO: Add link to wasm

Topological matching means that the pattern has to be deformable in the match
without tearing. In the example above the exactly shape of the orange and gray
blobs as well as the dark gray arrow between them don't match exactly

## Turing machine example

![Turing machine](readme_resources/turing.gif)

This example uses more complex features: Solid regions, placeholders.

Features

- Topological patterns
- Solid regions, are matched exactly, no deformation allowed
- Sleeping regions are woken up at the end of a tick
- Placeholders
