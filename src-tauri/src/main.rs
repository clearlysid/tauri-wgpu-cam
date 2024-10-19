// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod webgpu;

use nokhwa::Buffer;
use std::{sync::Arc, time::Instant};
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};
use webgpu::WgpuState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the main window
            let window = app.get_webview_window("main").unwrap();

            // Create a WgpuState (containing the device, instance, adapter etc.)
            // And store it in the state
            let wgpu_state = async_runtime::block_on(WgpuState::new(window));
            app.manage(Arc::new(wgpu_state));

            // Create a channel for sending/receiving buffers from the camera
            let (tx, rx) = std::sync::mpsc::channel::<Buffer>();

            let app_handle = app.app_handle().clone();

            // Spawn a thread for the camera
            async_runtime::spawn(async move {
                let mut camera = utils::create_camera();

                camera.open_stream().expect("Could not open stream");

                std::thread::sleep(std::time::Duration::from_secs(1));

                for i in 0..1000 {
                    let buffer = camera.frame().expect("Could not get frame");
                    tx.send(buffer).expect("Could not send buffer");
                    println!("Frame {i} sent");
                }

                camera.stop_stream().expect("Could not stop stream");
                println!("Camera Stream Stopped");
            });

            async_runtime::spawn(async move {
                let wgpu_state = app_handle.state::<Arc<WgpuState>>();

                while let Ok(buffer) = rx.recv() {
                    let t = Instant::now();

                    let bytes = buffer.buffer();
                    let width = buffer.resolution().width();
                    let height = buffer.resolution().height();

                    // Approach 1 (SLOW)
                    // Decode RgbAFormat via Nokhwa
                    // let rgba_bytes = buffer.decode_image::<RgbAFormat>().unwrap();

                    // Approach 2 (FAST)
                    // Convert YUYV to RGBA using Rayon
                    // let rgba_bytes = utils::yuyv_to_rgba(bytes, width as usize, height as usize);

                    // Approach 3 (FASTEST)
                    // Convert YUYV to RGBA using wgpu compute shader
                    wgpu_state.render_yuyv_bytes(bytes, width, height);

                    println!("Frame rendered in: {}ms", t.elapsed().as_millis());
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                RunEvent::WebviewEvent { label, event, .. } => {
                    println!("Received event from {}: {:?}", label, event);
                }
                RunEvent::WindowEvent {
                    label: _,
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let wgpu_state = app_handle.state::<Arc<WgpuState>>();

                    let mut config = wgpu_state.config.lock().unwrap();
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    wgpu_state.surface.configure(&wgpu_state.device, &config);

                    // TODO: Request redraw on macos (not exposed in tauri yet).
                }
                _ => (),
            }
        });
}
