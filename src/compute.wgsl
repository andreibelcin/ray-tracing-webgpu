@group(0) @binding(0)
var output_tex: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(0)
var<uniform> camera_origin: vec3f;

@group(1) @binding(1)
var<uniform> viewport_du: vec3f;
@group(1) @binding(2)
var<uniform> viewport_dv: vec3f;

@group(1) @binding(3)
var<uniform> pixel_00_center: vec3f;

@compute @workgroup_size(1)
fn main(
    @builtin(global_invocation_id)
    id: vec3u,
) {
    let pixel_center = pixel_00_center + (f32(id.x) * viewport_du) + (f32(id.y) * viewport_dv);
    let ray_direction = normalize(pixel_center - camera_origin);

    let a = 0.5 * (ray_direction.y + 1.0); 
    let color = vec4f((1.0 - a) * vec3f(1.0, 1.0, 1.0) + a * vec3f(0.0, 0.0, 1.0), 1.0);
    textureStore(output_tex, id.xy, color);
}
