use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{RequestedFormat, RequestedFormatType, Resolution};
use nokhwa::{native_api_backend, query, Camera};

use rayon::prelude::*;

pub fn create_camera() -> Camera {
    let backend = native_api_backend().expect("Could not get backend");
    let devices = query(backend).expect("Could not query backend");
    let device = devices.first().expect("No devices found");

    let format =
        RequestedFormat::new::<RgbAFormat>(RequestedFormatType::HighestResolution(Resolution {
            width_x: 1280,
            height_y: 720,
        }));

    Camera::new(device.index().to_owned(), format).expect("Could not create camera")
}

/// HIGH PERFORMANCE DECODING
pub fn yuyv_to_rgba(yuyv: &[u8], width: usize, height: usize) -> Vec<u8> {
    let pixel_count = width * height;
    let mut rgba = vec![0u8; pixel_count * 4];

    rgba.par_chunks_exact_mut(4)
        .enumerate()
        .for_each(|(i, chunk)| {
            let yuyv_index = (i / 2) * 4;
            let y = if i % 2 == 0 {
                yuyv[yuyv_index]
            } else {
                yuyv[yuyv_index + 2]
            };
            let u = yuyv[yuyv_index + 1];
            let v = yuyv[yuyv_index + 3];

            let c = y as f32 - 16.0;
            let d = u as f32 - 128.0;
            let e = v as f32 - 128.0;

            let r = (298.0 * c + 409.0 * e + 128.0) / 256.0;
            let g = (298.0 * c - 100.0 * d - 208.0 * e + 128.0) / 256.0;
            let b = (298.0 * c + 516.0 * d + 128.0) / 256.0;

            chunk[0] = r.clamp(0.0, 255.0) as u8;
            chunk[1] = g.clamp(0.0, 255.0) as u8;
            chunk[2] = b.clamp(0.0, 255.0) as u8;
            chunk[3] = 255; // Alpha channel (fully opaque)
        });

    rgba
}
