precision mediump float;
varying vec2 v_coords;
uniform vec2 size;
uniform float alpha;
uniform vec4 border_color;
uniform float border_thickness;
uniform float corner_radius;

float rounded_rect_SDF(
    vec2 point,
    vec2 half_rect_size,
    float corner_radius
) {
    vec2 distance_to_corner = abs(point) - half_rect_size + corner_radius;

    vec2 outside_distance = max(distance_to_corner, 0.0);

    float outside_corner_distance = length(outside_distance);
    float inside_rect_distance = min(
        max(distance_to_corner.x, distance_to_corner.y),
        0.0
    );

    return outside_corner_distance + inside_rect_distance - corner_radius;
}

void main() {
    vec2 centered_pos = v_coords * size - size * 0.5;

    float dist_outer = rounded_rect_SDF(
        centered_pos,
        size * 0.5,
        corner_radius
    );

    float dist_inner = rounded_rect_SDF(
        centered_pos,
        size * 0.5 - vec2(border_thickness, border_thickness),
        corner_radius - border_thickness
    );

    if (dist_outer > 0.0 || dist_inner < 0.0) {
        discard;
    }

    // gl_FragColor = vec4(v_coords, 1.0, 1.0);
    gl_FragColor = border_color * alpha;
}