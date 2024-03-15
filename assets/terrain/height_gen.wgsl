
@group(0) @binding(0) var image: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1) var<uniform> offset: vec2<f32>;
@group(0) @binding(2) var<uniform> scale: f32;

@compute @workgroup_size(8, 8, 1)
fn main(
    @builtin(global_invocation_id) invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>
) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let uv_scale = vec2<f32>(f32(num_workgroups.x) * 8.0, f32(num_workgroups.y) * 8.0);
    let uv = vec2<f32>(f32(invocation_id.x) / uv_scale.x, 1.0 - f32(invocation_id.y) / uv_scale.y);
    
    let p = uv * scale + offset;
    var h = 0.0;
    
    // create our height, it could be anything
    h = fbm_gray(p, 8u);

    //h = abs(simplexNoise2(p));
    
    // Some good resources for noise algorithms
    // Inigo Quilez - https://iquilezles.org/articles/
    // WGSL Noise Algorithms - https://gist.github.com/munrocket/236ed5ba7e409b8bdf1ff6eca5dcdc39
    // Noisy Bevy - https://github.com/johanhelsing/noisy_bevy
    
    //var pi = 3.141592653589793;   
    //h = sin(p.x * pi) * 0.5 + 0.5;

    // we only use the red channel for terrain generation, but since we
    // are using same texture for the material, we will use grayscale
    let c = vec4<f32>(h, h, h, 1.0); 

    textureStore(image, location, c);
}

fn fbm_gray(in: vec2f, num_octaves: u32) -> f32 {
    var total: f32 = 0.0;
    var frequency: f32 = 1.0;
    var amplitude: f32 = 1.0;    
    
    // for loop guard, is there a better way to do this?
    let max_iterations: u32 = 10u; 
    for (var i: u32 = 0; i < max_iterations; i++) {
        if (i >= num_octaves) {
            break;
        } 
        total += simplexNoise2(in * frequency) * amplitude;
        frequency *= 2.0;
        amplitude *= 0.3;
        // since converting to grayscale
        // we want the biggest contribution to be positive
        if (i == 0) {
            total = abs(total);
        }
    }

    return total;
}

//  MIT License. © Ian McEwan, Stefan Gustavson, Munrocket
fn simplexNoise2(v: vec2f) -> f32 {
    let C = vec4(
        0.211324865405187, // (3.0-sqrt(3.0))/6.0
        0.366025403784439, // 0.5*(sqrt(3.0)-1.0)
        -0.577350269189626, // -1.0 + 2.0 * C.x
        0.024390243902439 // 1.0 / 41.0
    );

    // First corner
    var i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    // Other corners
    var i1 = select(vec2(0., 1.), vec2(1., 0.), x0.x > x0.y);

    // x0 = x0 - 0.0 + 0.0 * C.xx ;
    // x1 = x0 - i1 + 1.0 * C.xx ;
    // x2 = x0 - 1.0 + 2.0 * C.xx ;
    var x12 = x0.xyxy + C.xxzz;
    x12.x = x12.x - i1.x;
    x12.y = x12.y - i1.y;

    // Permutations
    i = mod289(i); // Avoid truncation effects in permutation

    var p = permute3(permute3(i.y + vec3(0., i1.y, 1.)) + i.x + vec3(0., i1.x, 1.));
    var m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3(0.));
    m *= m;
    m *= m;

    // Gradients: 41 points uniformly over a line, mapped onto a diamond.
    // The ring size 17*17 = 289 is close to a multiple of 41 (41*7 = 287)
    let x = 2. * fract(p * C.www) - 1.;
    let h = abs(x) - 0.5;
    let ox = floor(x + 0.5);
    let a0 = x - ox;

    // Normalize gradients implicitly by scaling m
    // Approximation of: m *= inversesqrt( a0*a0 + h*h );
    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);

    // Compute final noise value at P
    let g = vec3(a0.x * x0.x + h.x * x0.y, a0.yz * x12.xz + h.yz * x12.yw);
    return 130. * dot(m, g);
}

fn mod289(x: vec2f) -> vec2f {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn mod289_3(x: vec3f) -> vec3f {
    return x - floor(x * (1. / 289.)) * 289.;
}

fn permute3(x: vec3f) -> vec3f {
    return mod289_3(((x * 34.) + 1.) * x);
}