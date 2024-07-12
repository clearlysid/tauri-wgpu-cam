// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod webgpu;

use nokhwa::pixel_format::RgbAFormat;
use std::sync::Arc;
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};
use webgpu::WgpuState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let wgpu_state = async_runtime::block_on(WgpuState::new(window));
            app.manage(Arc::new(wgpu_state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                RunEvent::Ready => {
                    let app_clone = app_handle.clone();

                    async_runtime::spawn(async move {
                        let mut camera = utils::create_camera();

                        // Open the camera stream
                        camera.open_stream().expect("Could not open stream");

                        std::thread::sleep(std::time::Duration::from_secs(1));

                        for i in 0..100 {
                            let buffer = camera.frame().expect("Could not get frame");
                            let wgpu_state = app_clone.state::<Arc<WgpuState>>();

                            let frame = buffer
                                .decode_image::<RgbAFormat>()
                                .expect("Could not decode frame");

                            let texture_size = wgpu::Extent3d {
                                width: frame.width(),
                                height: frame.height(),
                                depth_or_array_layers: 1,
                            };

                            let texture =
                                wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
                                    label: None,
                                    sample_count: 1,
                                    mip_level_count: 1,
                                    size: texture_size,
                                    dimension: wgpu::TextureDimension::D2,
                                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                                        | wgpu::TextureUsages::COPY_DST,
                                    view_formats: &[],
                                });

                            wgpu_state.queue.write_texture(
                                wgpu::ImageCopyTexture {
                                    texture: &texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &frame,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4 * frame.width()),
                                    rows_per_image: Some(frame.height()),
                                },
                                texture_size,
                            );

                            let texture_view =
                                texture.create_view(&wgpu::TextureViewDescriptor::default());
                            let bind_group =
                                wgpu_state
                                    .device
                                    .create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: &wgpu_state.bind_group_layout,
                                        entries: &[
                                            wgpu::BindGroupEntry {
                                                binding: 0,
                                                resource: wgpu::BindingResource::TextureView(
                                                    &texture_view,
                                                ),
                                            },
                                            wgpu::BindGroupEntry {
                                                binding: 1,
                                                resource: wgpu::BindingResource::Sampler(
                                                    &wgpu_state.sampler,
                                                ),
                                            },
                                        ],
                                        label: None,
                                    });

                            let output = wgpu_state
                                .surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder = wgpu_state.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor { label: None },
                            );
                            {
                                let mut rpass =
                                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: None,
                                        color_attachments: &[Some(
                                            wgpu::RenderPassColorAttachment {
                                                view: &view,
                                                resolve_target: None,
                                                ops: wgpu::Operations {
                                                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                                    store: wgpu::StoreOp::Store,
                                                },
                                            },
                                        )],
                                        depth_stencil_attachment: None,
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                    });
                                rpass.set_pipeline(&wgpu_state.render_pipeline);
                                rpass.set_bind_group(0, &bind_group, &[]);
                                rpass.draw(0..6, 0..1);
                            }

                            wgpu_state.queue.submit(Some(encoder.finish()));
                            output.present();

                            println!("Frame {i}");
                        }

                        camera.stop_stream().expect("Could not stop stream");
                        println!("Camera Stream Stopped");
                    });
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

                    // TODO: Request redraw on macos (not exposed in taurip yet).
                }
                RunEvent::MainEventsCleared => {
                    // println!("MainEventsCleared");
                }
                _ => (),
            }
        });
}
