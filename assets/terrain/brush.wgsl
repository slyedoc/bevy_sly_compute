
@group(0) @binding(0) var height_map: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var<uniform> radius: f32; // in uv space of height map
@group(0) @binding(2) var<uniform> position: vec2<f32>; // in uv space of height map
@group(0) @binding(3) var<uniform> strength: f32; // how much to add to the height map, can be negative

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) { 
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let uv_scale = vec2<f32>(f32(num_workgroups.x) * 8.0, f32(num_workgroups.y) * 8.0);
    let uv = vec2<f32>(f32(invocation_id.x) / uv_scale.x, 1.0 - f32(invocation_id.y) / uv_scale.y);

    // find the current height at the location
    let current = textureLoad(height_map, location);

    // see if we are within the brush radius
    let brush_distance = distance(uv, position) / radius;

    if (brush_distance < 1.0) {
        // if we are, lets change the height
        let falloff = 1.0 - smoothstep(0.0, 1.0, brush_distance);
        let height = current.x + falloff * strength;

        // again, only using red channel for height, but since we are displaying the texture we will use grayscale
        textureStore(height_map, location, vec4<f32>(height, height, height, 1.0));
    }
}
