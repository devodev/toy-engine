#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

// uniforms
layout (binding = 0) uniform Matrices {
    mat4 ortho;
} matrices;

// inputs
layout (location = 0) in vec2 vPos;
layout (location = 1) in vec2 vUv;
layout (location = 2) in vec4 vColor;

// outputs
layout (location = 0) out vec2 oUV;
layout (location = 1) out vec4 oColor;

// Converts a color from linear light gamma to sRGB gamma
// https://gamedev.stackexchange.com/a/148088
vec4 fromLinear(vec4 linearRGB) {
    bvec3 cutoff = lessThan(linearRGB.rgb, vec3(0.0031308));
    vec3 higher = vec3(1.055)*pow(linearRGB.rgb, vec3(1.0/2.4)) - vec3(0.055);
    vec3 lower = linearRGB.rgb * vec3(12.92);
    return vec4(mix(higher, lower, cutoff), linearRGB.a);
}

// Converts a color from sRGB gamma to linear light gamma
// https://gamedev.stackexchange.com/a/148088
vec4 toLinear(vec4 sRGB) {
    bvec3 cutoff = lessThan(sRGB.rgb, vec3(0.04045));
    vec3 higher = pow((sRGB.rgb + vec3(0.055))/vec3(1.055), vec3(2.4));
    vec3 lower = sRGB.rgb/vec3(12.92);
    return vec4(mix(higher, lower, cutoff), sRGB.a);
}

void main() {
    oUV = vUv;
    oColor = vColor;
    // The rule of thumb for color space handling is to keep the sRGB color storage format
    // in an array of 4 bytes (values 0-255), and linear-space color storage format in an
    // array of 4 floats (values 0.0-1.0). Any conversion between these formats must go
    // through gamma correction.
    // (e.g. the pow(n / 255.0, 2.2) and pow(n, 1.0 / 2.2) * 255.0 approximations)
    //
    // https://github.com/ocornut/imgui/issues/1724
    // https://github.com/ocornut/imgui/issues/578
    oColor = toLinear(oColor);
    gl_Position = matrices.ortho * vec4(vPos.x, vPos.y, 0.0, 1.0);
}
