#version 300 es

precision mediump float;
in vec2 v_uv;
out vec4 o_color;

uniform sampler2D u_tex;
uniform vec2 u_src_size;
uniform float u_time;

float scanline(float y) {
    return 0.85 + 0.15 * sin(y * 3.14159 * u_src_size.y);
}

float shadow_mask(vec2 p) {
    float x = floor(p.x);
    float m = mod(x, 3.0);
    return (m == 0.0) ? 1.00 : (m == 1.0 ? 0.92 : 0.88);
}

void main() {
    vec2 p = v_uv * u_src_size;
    vec2 ip = (floor(p) + 0.5) / u_src_size;
    vec3 c = texture(u_tex, ip).rgb;

    c *= scanline(p.y);
    c *= shadow_mask(p);

    vec2 d = v_uv - 0.5;
    float vig = smoothstep(0.9, 0.2, dot(d, d));
    c *= vig;

    o_color = vec4(c, 1.0);
}