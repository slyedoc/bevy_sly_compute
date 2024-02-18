
@group(0) @binding(0) var<uniform> add: f32;

@group(0) @binding(1) var<storage, read_write> my_storage: array<f32>;

@group(0) @binding(2) var<uniform> color: vec4<f32>;

@group(0) @binding(3) var image: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(1)
fn pre(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    my_storage[invocation_id.x] = my_storage[invocation_id.x] + add;
}

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let uv_scale = vec2<f32>(f32(num_workgroups.x) * 8.0, f32(num_workgroups.y) * 8.0);
    let uv = vec2<f32>(f32(invocation_id.x) / uv_scale.x, 1.0 - f32(invocation_id.y) / uv_scale.y);
   
    let c = vec4<f32>(uv.x, uv.y, 0.0, 1.0);

    textureStore(image, location, c);
}