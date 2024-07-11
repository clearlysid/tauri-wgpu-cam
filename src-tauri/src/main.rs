// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;

use nokhwa::pixel_format::RgbAFormat;

use std::sync::Mutex;
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Step 1: Create a window
            let window = app.get_webview_window("main").unwrap();
            let size = window.inner_size()?;

            // Step 2: Create a WGPU instance, surface and adapter
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });

            let surface = instance.create_surface(window).unwrap();
            let adapter =
                async_runtime::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                }))
                .expect("Failed to find an appropriate adapter");

            // Create the logical device and command queue
            let (device, queue) = async_runtime::block_on(
                adapter.request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                ),
            )
            .expect("Failed to create device");

            // Load the shaders from disk
            let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

            let swapchain_capabilities = surface.get_capabilities(&adapter);
            let swapchain_format = swapchain_capabilities.formats[0];

            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions {
                        ..Default::default()
                    },
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                    compilation_options: wgpu::PipelineCompilationOptions {
                        ..Default::default()
                    },
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: swapchain_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: swapchain_capabilities.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            surface.configure(&device, &config);

            app.manage(surface);
            app.manage(render_pipeline);
            app.manage(device);
            app.manage(queue);
            app.manage(Mutex::new(config));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                RunEvent::Ready => {
                    println!("Ready");

                    let app_clone = app_handle.clone();

                    async_runtime::spawn(async move {
                        let mut camera = utils::create_camera();

                        // Open the camera stream
                        camera.open_stream().expect("Could not open stream");

                        // wait 100ms
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        for i in 0..100 {
                            // let frame = camera.frame().expect("Could not get frame");
                            // let format = frame.source_frame_format().to_string();
                            // println!("Frame {i}: {format}");

                            let device = app_clone.state();
                            let queue = app_clone.state();

                            let wgpu_frame = camera
                                .frame_texture::<RgbAFormat>(&device, &queue, None)
                                .expect("couldn't get texture");

                            println!("Frame {i}: {:?}", wgpu_frame.width());
                        }

                        camera.stop_stream().expect("Could not stop stream");
                    });
                }
                RunEvent::WindowEvent {
                    label: _,
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let config = app_handle.state::<Mutex<wgpu::SurfaceConfiguration>>();
                    let surface = app_handle.state::<wgpu::Surface>();
                    let device = app_handle.state::<wgpu::Device>();

                    let mut config = config.lock().unwrap();
                    config.width = if size.width > 0 { size.width } else { 1 };
                    config.height = if size.height > 0 { size.height } else { 1 };
                    surface.configure(&device, &config)

                    // TODO: Request redraw on macos (not exposed in tauri yet).
                }
                RunEvent::MainEventsCleared => {
                    println!("MainEventsCleared");

                    let surface = app_handle.state::<wgpu::Surface>();
                    let render_pipeline = app_handle.state::<wgpu::RenderPipeline>();
                    let device = app_handle.state::<wgpu::Device>();
                    let queue = app_handle.state::<wgpu::Queue>();

                    let frame = surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                _ => (),
            }
        });
}
