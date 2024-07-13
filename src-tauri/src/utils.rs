use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, query, Camera};

pub fn create_camera() -> Camera {
    let backend = native_api_backend().expect("Could not get backend");
    let devices = query(backend).expect("Could not query backend");
    let device = devices.first().expect("No devices found");

    let format = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestResolution);

    Camera::new(device.index().to_owned(), format).expect("Could not create camera")
}

// Format conversion needed because nokhwa is broken on mac
pub fn uyvy_to_rgba(in_buf: &[u8], out_buf: &mut [u8]) {
    debug_assert!(out_buf.len() == in_buf.len() * 2);

    for (in_chunk, out_chunk) in in_buf.chunks_exact(4).zip(out_buf.chunks_exact_mut(8)) {
        let u0 = in_chunk[1];
        let y0 = in_chunk[0];
        let v0 = in_chunk[3];
        let y1 = in_chunk[2];

        let (r1, g1, b1) = ycbcr_to_rgb(y0, u0, v0);
        out_chunk[0] = r1;
        out_chunk[1] = g1;
        out_chunk[2] = b1;
        out_chunk[3] = 255; // Alpha channel

        let (r2, g2, b2) = ycbcr_to_rgb(y1, u0, v0);
        out_chunk[4] = r2;
        out_chunk[5] = g2;
        out_chunk[6] = b2;
        out_chunk[7] = 255; // Alpha channel
    }
}

#[inline]
fn ycbcr_to_rgb(y: u8, cb: u8, cr: u8) -> (u8, u8, u8) {
    let y = y as f32;
    let cb = cb as f32 - 128.0;
    let cr = cr as f32 - 128.0;

    // rec 709: https://mymusing.co/bt-709-yuv-to-rgb-conversion-color/
    let r = y + 1.5748 * cr;
    let g = y - 0.187324 * cb - 0.468124 * cr;
    let b = y + 1.8556 * cb;

    (clamp(r), clamp(g), clamp(b))
}

#[inline]
fn clamp(val: f32) -> u8 {
    if val < 0.0 {
        0
    } else if val > 255.0 {
        255
    } else {
        val.round() as u8
    }
}
