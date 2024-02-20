
@group(0) @binding(0) var<uniform> scale: f32;

@group(0) @binding(1) var image: texture_storage_2d<rgba8unorm, read_write>;

@group(0) @binding(2) var<uniform> world_scale: u32;


@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    let uv_scale = vec2<f32>(f32(num_workgroups.x) * 8.0, f32(num_workgroups.y) * 8.0);
    let uv = vec2<f32>(f32(invocation_id.x) / uv_scale.x, 1.0 - f32(invocation_id.y) / uv_scale.y);
    var pi = 3.141592653589793;   

    //let c = vec4<f32>(sin(uv.x), 0.0, 0.0, 1.0);
    //let c = vec4<f32>(uv.x, uv.y, 0.0, 1.0);    

    // sin waves
    let wave = sin(uv.x * pi * scale) * 0.5 + 0.5;
    let c = vec4<f32>(wave, 0.0, 0.0, 1.0);

    textureStore(image, location, c);
}