Could the drawing be more exact by receiving mouse updates at 120Hz instead
of 60, while still drawing at 60Hz.

`egui::Event::MouseMoved` is the raw input event without mouse acceleration and
is received at a much higher rate on Windows, probably USB polling rate.

The refresh rate of can be changed by setting vsync to false in 
`eframe::NativeOptions` and calling 
`ctx.request_repaint_after_secs(1.0f32/120.0f32);`

Would it even make a noticeable difference?