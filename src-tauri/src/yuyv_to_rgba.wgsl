@group(0) @binding(0) var<storage, read> yuyv_buffer: array<u32>;
@group(0) @binding(1) var<storage, read_write> rgba_buffer: array<u32>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let width = 1920u; // Replace with the actual width of your image
    let height = 1080u; // Replace with the actual height of your image

    let x = global_id.x;
    let y = global_id.y;

    if (x >= width || y >= height) {
        return;
    }

    let index = y * width + x;
    let yuyv_index = (index / 2u) * 4u;

    let yuyv = yuyv_buffer[yuyv_index / 4u];
    let yuyv_bytes = vec4<u32>(
        (yuyv & 0xFFu),
        ((yuyv >> 8) & 0xFFu),
        ((yuyv >> 16) & 0xFFu),
        ((yuyv >> 24) & 0xFFu)
    );

    var y_val: u32;
    if (index % 2u == 0u) {
        y_val = yuyv_bytes.x;
    } else {
        y_val = yuyv_bytes.z;
    }
    let u = yuyv_bytes.y;
    let v = yuyv_bytes.w;

    let c = f32(y_val) - 16.0;
    let d = f32(u) - 128.0;
    let e = f32(v) - 128.0;

    let r = (298.0 * c + 409.0 * e + 128.0) / 256.0;
    let g = (298.0 * c - 100.0 * d - 208.0 * e + 128.0) / 256.0;
    let b = (298.0 * c + 516.0 * d + 128.0) / 256.0;

    let rgba = vec4<u32>(
        u32(clamp(r, 0.0, 255.0)),
        u32(clamp(g, 0.0, 255.0)),
        u32(clamp(b, 0.0, 255.0)),
        255u
    );

    rgba_buffer[index] = (rgba.x & 0xFFu) | ((rgba.y & 0xFFu) << 8) | ((rgba.z & 0xFFu) << 16) | ((rgba.w & 0xFFu) << 24);
}
