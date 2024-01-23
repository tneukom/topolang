eframe glow web seems to clear the whole framebuffer before rendering, which is after
update() is called, so we cannot draw during update(). Probably not intended, maybe make
an issue.

https://github.com/emilk/egui/blob/08fb447fb55293b2d49343cf5ade2c59d436bc58/crates/eframe/src/web/web_painter_glow.rs#L62

## Possible workarounds
- Replace `WebPainterGlow` with a custom implementation, that doesn't clear at all? 
Would require forking eframe which is part of the egui repo

- Rendering in an egui callback, 
  - see [PaintCallback example](https://github.com/emilk/egui/blob/master/crates/egui_demo_app/src/apps/custom3d_glow.rs).
  - Before calling the callback [gl.viewport is called](https://github.com/emilk/egui/blob/5388e656234dcded437fb3905d5cba98ae2e6681/crates/egui_glow/src/painter.rs#L408)
  - See [gl.viewport](https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/viewport)


Related:
- [Order of clearing and painting change in 0.24?](https://github.com/emilk/egui/issues/3659)
- [Drawing 3D scene to a canvas with WGPU](https://github.com/emilk/egui/discussions/1661)
- 