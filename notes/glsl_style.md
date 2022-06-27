# In vertex shader:
inputs: in prefix
outputs: frag prefix

# In fragment shader:
inputs: frag prefix
outputs out prefix

# General:
uniforms: uniform prefx

Why use prefixes at all?
- No IDE to refactor, for example having line_a as a uniform and function argument
  how to rename the uniform?
- No IDE to show where a variable is from
