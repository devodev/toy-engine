#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// uniforms
layout (binding = 0) uniform UBO {
    mat4 vp;
} ubo;

// inputs
layout (location = 0) in vec4 vPos;
layout (location = 1) in vec4 vColor;

// outputs
layout (location = 0) out vec4 color;

void main() {
    //color = vPos;
    color = vColor;
    gl_Position = ubo.vp * vPos;
}
