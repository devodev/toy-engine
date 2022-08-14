#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// inputs
layout (location = 0) in vec4 color;

// outputs
layout (location = 0) out vec4 uFragColor;

void main() {
    uFragColor = color;
}
