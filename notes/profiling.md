Tracy
=====

Each zone can have a color and additional information as a string.

Rust bindings:

- https://crates.io/crates/tracy-client: Supports emitting [extra info](
  https://docs.rs/tracy-client/latest/tracy_client/struct.Span.html#method.emit_text).
- https://crates.io/crates/tracy_full: Doesn't seem to support extra info.
- https://crates.io/crates/profiling: Supports multiple profilers, including
  tracy, puffin. Supports extra text but [not color](
  https://github.com/aclysma/profiling/issues/19)
- https://crates.io/crates/tracing: Maybe a bit too much

Links:
https://news.ycombinator.com/item?id=34559993

Conclusion: Use tracy-client, supports extra info and color


Superliminal
============

Similar to Tracy, has GUI tool to display traces

https://superluminal.eu/

Rust bindings: https://docs.rs/superluminal-perf/latest/superluminal_perf/

Puffin
======

https://github.com/EmbarkStudios/puffin

Custom tracing for search
=========================

Observer trait for search events.
