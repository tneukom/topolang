https://unlimited3d.wordpress.com/2020/01/08/srgb-color-space-in-opengl/

> WebGL 2.0, for instance, being an analog of OpenGL ES 3.0 for web, also
> supports sRGB textures and FBOs, but does not provide any means of color space
> control for a canvas element. In fact, the canvas buffer is treated as a
> pass-through RGB buffer, so that sRGB offscreen FBO blitting into the window
> buffer (canvas) causes a color shift. This problem is currently under
> discussion
> within the canvas color space proposal.

https://stackoverflow.com/questions/56204242/do-i-need-output-gamma-correction-in-a-fragment-shader

> When I output a color via fragColor in the main() function of a fragment
> shader, are the color component intensities interpreted as linear RGB or
> sRGB? ...

> Every fragment shader output is routed to a specific image in the
> framebuffer (based on glDrawBuffers state). That image has a format. That
> format
> defines the default colorspace interpretation of the corresponding output.
> Therefore, if you are writing a value to an image in a linear RGB format, then
> the system will interpret that value as a linear RGB value. If you are writing
> a
> value to an image in an sRGB format, then the system will interpret that value
> as already being in the sRGB colorspace ...

https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/getFramebufferAttachmentParameter

Use to read FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING, looks like this function is
not mapped by glow.

https://hackmd.io/@jgilbert/sRGB-WebGL

https://learnopengl.com/Advanced-Lighting/Gamma-Correction

https://stackoverflow.com/questions/11386199/when-to-call-glenablegl-framebuffer-srgb

> When GL_FRAMEBUFFER_SRGB is enabled, all writes to an image with an sRGB image
> format will assume that the input colors (the colors being written) are in a
> linear colorspace. Therefore, it will convert them to the sRGB colorspace.

https://gamedev.net/forums/topic/697254-gamma-correction-confusion/5381563/

### srgb to linear and back

https://gamedev.stackexchange.com/questions/92015/optimized-linear-to-srgb-glsl
https://en.wikipedia.org/wiki/SRGB

### Conclusion

- GL_FRAMEBUFFER_SRGB is not supported on WebGl, at least there's no mention of
  it under https://registry.khronos.org/webgl/specs/latest/2.0/
- Is the Webgl canvas framebuffer always srgb?
- Passing sRGB as output in the pixel shader seems to work on desktop and web.

For now, we just use RGB texture so the lookup does no conversion and directly
return that color. However:

- Mipmaps might be off
- As soon as we do any calculations / blending using sRGB colors won't work
  anymore
