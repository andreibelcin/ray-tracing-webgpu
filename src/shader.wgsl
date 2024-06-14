@group(0) @binding(0)
var compute_tex: texture_2d<f32>;

@group(0) @binding(1)
var compute_sampler: sampler;

struct VertexOut {
    @builtin(position) pos: vec4f,
    @location(0) tex_coord: vec2f,
}

@vertex
fn vert_main(
    @builtin(vertex_index) i: u32,
) -> VertexOut {
    var positions = array(
        // 1st triangle
        vec2f(1.0, 1.0), 
        vec2f(1.0, -1.0),
        vec2f(-1.0, -1.0),

        // 2st triangle
        vec2f(-1.0, -1.0),
        vec2f(-1.0, 1.0),
        vec2f(1.0, 1.0),
    );

    let pos = positions[i];
    
    var out: VertexOut;
    out.pos = vec4f(pos, 0.0, 1.0);
    out.tex_coord = vec2f((pos.x + 1.0) * 0.5, (1.0 - pos.y) * 0.5);
    return out;
}

@fragment
fn frag_main(
    vert_out: VertexOut,
) -> @location(0) vec4f {
    return textureSample(compute_tex, compute_sampler, vert_out.tex_coord);
}
