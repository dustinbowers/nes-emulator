#version 330 core
in vec2 v_uv;
out vec4 o_color;

uniform sampler2D u_tex;
uniform vec2 u_src_size; // 256,240
uniform float u_time;

vec3 sample_nearest(vec2 p_px) {
    vec2 ip = (floor(p_px) + 0.5) / u_src_size;
    return texture(u_tex, ip).rgb;
}

float scanline(float y_px) {
    float s = 0.5 + 0.5 * sin(y_px * 3.14159);
    return mix(0.82, 1.0, s);
}

vec3 shadow_mask(float x_px) {
    float m = mod(floor(x_px), 3.0);
    vec3 mask = vec3(1.0);
    if (m == 0.0) mask = vec3(1.00, 0.92, 0.92);
    if (m == 1.0) mask = vec3(0.92, 1.00, 0.92);
    if (m == 2.0) mask = vec3(0.92, 0.92, 1.00);
    return mix(vec3(1.0), mask, 0.35); // strength
}

void main() {
    // mild barrel distortion
    vec2 uv = v_uv * 2.0 - 1.0; // -1..1
    float r2 = dot(uv, uv);
    // uv *= 1.0 + 0.08 * r2;      // curvature strength (try 0.05–0.12)
    uv *= 1.0 + 0.03 * r2;      // curvature strength (try 0.05–0.12)
    uv = uv * 0.5 + 0.5;        // back to 0..1

    // discard outside tube
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        o_color = vec4(0.0);
        return;
    }

    vec2 p = uv * u_src_size;

    vec3 c0 = sample_nearest(p);
    vec3 c1 = sample_nearest(p + vec2(1.0, 0.0));


    // Scanline
    float luma = dot(c0, vec3(0.299, 0.587, 0.114));
    float bleed = smoothstep(0.6, 1.0, luma) * 0.10;
    vec3 c = mix(c0, c1, bleed);

    float s = scanline(p.y);

    // brightness of the pixel (0..1)
    float lum = dot(c, vec3(0.299, 0.587, 0.114));

    // bright pixels reduce scanline darkness
    s = mix(s, 1.0, lum * 0.5);

    c *= s;
    c *= shadow_mask(p.x);

    vec2 d = uv - 0.5;
    float vig = 1.0 - 0.5 * dot(d, d);
    c *= clamp(vig, 0.0, 1.0);

    o_color = vec4(c, 1.0);
}
