#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// uniforms
layout (binding = 1, set = 0) uniform sampler2D fontsSampler;

// inputs
layout (location = 0) in vec2 vUv;
layout (location = 1) in vec4 vColor;

// outputs
layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = vColor * texture(fontsSampler, vUv);
}
