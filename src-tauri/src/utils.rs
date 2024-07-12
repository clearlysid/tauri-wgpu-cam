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
