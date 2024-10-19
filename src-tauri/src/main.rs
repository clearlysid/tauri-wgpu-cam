// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;
mod webgpu;

use nokhwa::Buffer;
use std::{sync::Arc, time::Instant};
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};
use webgpu::WgpuState;
use wgpu::util::DeviceExt;

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

                    // TODO: this step is very slow
                    // let bytes = buffer
                    //     .decode_image::<RgbAFormat>()
                    //     .expect("Could not decode frame");

                    // let bytes = utils::yuyv_to_rgba(bytes, width as usize, height as usize);
                    let yuyv_buffer =
                        wgpu_state
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("YUYV Buffer"),
                                contents: bytes,
                                usage: wgpu::BufferUsages::STORAGE,
                            });

                    let rgba_buffer = wgpu_state.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("RGBA Buffer"),
                        size: (width * height * 4) as u64,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    });

                    let compute_bind_group_layout = wgpu_state.device.create_bind_group_layout(
                        &wgpu::BindGroupLayoutDescriptor {
                            entries: &[
                                wgpu::BindGroupLayoutEntry {
                                    binding: 0,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::Buffer {
                                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 1,
                                    visibility: wgpu::ShaderStages::COMPUTE,
                                    ty: wgpu::BindingType::Buffer {
                                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                            ],
                            label: Some("compute_bind_group_layout"),
                        },
                    );

                    let bind_group =
                        wgpu_state
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: &compute_bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: yuyv_buffer.as_entire_binding(),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: rgba_buffer.as_entire_binding(),
                                    },
                                ],
                                label: None,
                            });

                    let mut encoder =
                        wgpu_state
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Compute Encoder"),
                            });

                    {
                        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Compute Pass"),
                            timestamp_writes: None,
                        });
                        cpass.set_pipeline(&wgpu_state.compute_pipeline);
                        cpass.set_bind_group(0, &bind_group, &[]);
                        cpass.dispatch_workgroups((width + 15) / 16, (height + 15) / 16, 1);
                    }

                    let texture_size = wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    };

                    let texture = wgpu_state.device.create_texture(&wgpu::TextureDescriptor {
                        label: None,
                        sample_count: 1,
                        mip_level_count: 1,
                        size: texture_size,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });

                    encoder.copy_buffer_to_texture(
                        wgpu::ImageCopyBuffer {
                            buffer: &rgba_buffer,
                            layout: wgpu::ImageDataLayout {
                                offset: 0,
                                bytes_per_row: Some(4 * width),
                                rows_per_image: Some(height),
                            },
                        },
                        wgpu::ImageCopyTexture {
                            texture: &texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        texture_size,
                    );

                    wgpu_state.queue.submit(Some(encoder.finish()));

                    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let bind_group =
                        wgpu_state
                            .device
                            .create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: &wgpu_state.bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(&texture_view),
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

                    let mut encoder = wgpu_state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
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
