@group(0) @binding(0)
var output_tex: texture_storage_2d<rgba8unorm, write>;

@group(0) @binding(1)
var<uniform> resolution: vec2f;

@compute @workgroup_size(1)
fn main(
    @builtin(global_invocation_id)
    id: vec3u,
) {
    let color = vec4f(vec2f(id.xy) / resolution, 1.0 - f32(id.x) / resolution.x, 1.0);
    textureStore(output_tex, id.xy, color);
}
