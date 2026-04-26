struct ViewProjection {
    matrix: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> vp: ViewProjection;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) point_coord: vec2<f32>,
}

// Quad corner offsets for billboard sprite
var<private> CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
    vec2<f32>(-1.0,  1.0),
);

@vertex
fn vs_main(vertex: VertexInput, @builtin(vertex_index) v_idx: u32) -> VertexOutput {
    let corner = CORNERS[v_idx % 6u];
    let world_pos = vp.matrix * vertex.position;
    let clip_pos = world_pos + vec4<f32>(corner * vertex.size * 0.01, 0.0, 0.0);

    var out: VertexOutput;
    out.clip_position = clip_pos;
    out.color = vertex.color;
    out.point_coord = corner;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = length(in.point_coord);
    if dist > 1.0 {
        discard;
    }
    return in.color;
}
