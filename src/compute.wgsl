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

fn ray_at(ray_direction: vec3f, t: f32) -> vec3f {
    return camera_origin + t * ray_direction;
}

fn hit_sphere(center: vec3f, radius: f32, ray_direction: vec3f) -> f32 {
    let o = center - camera_origin;
    let a = dot(ray_direction, ray_direction);
    let h = dot(ray_direction, o);
    let c = dot(o, o) - radius * radius;
    let d = h * h - a * c;

    if d < 0.0 {
        return -1.0;
    } else {
        return (h - sqrt(d)) / a;
    }
}

fn get_color(ray_direction: vec3f) -> vec4f {
    let t = hit_sphere(vec3f(0.0, 0.0, -1.0), 0.5, ray_direction);
    if t > 0.0 {
        let n = normalize(ray_at(ray_direction, t) - vec3f(0.0, 0.0, -1.0));
        return vec4f((n + 1.0) * 0.5, 1.0);
    }

    let a = (ray_direction.y + 1.0) * 0.5;
    return vec4f((1.0 - a) * vec3f(0.8, 0.9, 1.0) + a * vec3f(0.1, 0.3, 1.0), 1.0);
}

@compute @workgroup_size(1)
fn main(
    @builtin(global_invocation_id)
    id: vec3u,
) {
    let pixel_center = pixel_00_center + (f32(id.x) * viewport_du) + (f32(id.y) * viewport_dv);
    let ray_direction = pixel_center - camera_origin;

    let color = get_color(ray_direction);
    // let rg = (ray_direction.xy + 1.0) * 0.5;
    // var b = 0.0;
    // if rg.x > 1.0 || rg.x < 0.0 {
    //     b = 1.0;
    // }

    // let color = vec4f(rg, b, 1.0);
    textureStore(output_tex, id.xy, color);
}
