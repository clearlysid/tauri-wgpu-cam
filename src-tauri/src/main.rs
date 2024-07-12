// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod utils;

use nokhwa::pixel_format::RgbAFormat;
use std::sync::Mutex;
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the main window dimensions
            let window = app.get_webview_window("main").unwrap();
            let size = window.inner_size()?;

            // Create a WGPU instance, adapter and surface (using window)
            let instance = wgpu::Instance::default();
            let surface = instance.create_surface(window).unwrap();
            let adapter =
                async_runtime::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                }))
                .expect("Failed to find an appropriate adapter");

            // Create a WGPU device and queue
            let (device, queue) = async_runtime::block_on(
                adapter.request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                ),
            )
            .expect("Failed to create device");

            // Load the shaders from disk
            let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                ..Default::default()
            });

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("bind_group_layout"),
                });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                push_constant_ranges: &[],
                // bind_group_layouts: &[],
                bind_group_layouts: &[&bind_group_layout],
            });

            let swapchain_capabilities = surface.get_capabilities(&adapter);
            let swapchain_format = swapchain_capabilities.formats[0];

            let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                multiview: None,
                depth_stencil: None,
                layout: Some(&pipeline_layout),
                primitive: wgpu::PrimitiveState::default(),
                multisample: wgpu::MultisampleState::default(),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
            });

            let config = wgpu::SurfaceConfiguration {
                width: size.width,
                height: size.height,
                view_formats: vec![],
                format: swapchain_format,
                desired_maximum_frame_latency: 2,
                present_mode: wgpu::PresentMode::Fifo,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                alpha_mode: swapchain_capabilities.alpha_modes[0],
            };

            surface.configure(&device, &config);

            app.manage(queue);
            app.manage(device);
            app.manage(surface);
            app.manage(render_pipeline);
            app.manage(Mutex::new(config));

            app.manage(sampler);
            app.manage(bind_group_layout);

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
                            let device = app_clone.state::<wgpu::Device>();
                            let queue = app_clone.state::<wgpu::Queue>();

                            let frame = buffer
                                .decode_image::<RgbAFormat>()
                                .expect("Could not decode frame");

                            let texture_size = wgpu::Extent3d {
                                width: frame.width(),
                                height: frame.height(),
                                depth_or_array_layers: 1,
                            };

                            let texture = device.create_texture(&wgpu::TextureDescriptor {
                                label: None,
                                sample_count: 1,
                                mip_level_count: 1,
                                size: texture_size,
                                view_formats: &[],
                                dimension: wgpu::TextureDimension::D2,
                                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                usage: wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_DST,
                            });

                            queue.write_texture(
                                wgpu::ImageCopyTexture {
                                    mip_level: 0,
                                    texture: &texture,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &frame,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    rows_per_image: Some(frame.height()),
                                    bytes_per_row: Some(4 * frame.width()),
                                },
                                texture_size,
                            );

                            let texture_view =
                                texture.create_view(&wgpu::TextureViewDescriptor::default());
                            let sampler = app_clone.state::<wgpu::Sampler>();
                            let bind_group_layout = app_clone.state::<wgpu::BindGroupLayout>();

                            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                                layout: &bind_group_layout,
                                entries: &[
                                    wgpu::BindGroupEntry {
                                        binding: 0,
                                        resource: wgpu::BindingResource::TextureView(&texture_view),
                                    },
                                    wgpu::BindGroupEntry {
                                        binding: 1,
                                        resource: wgpu::BindingResource::Sampler(&sampler),
                                    },
                                ],
                                label: None,
                            });

                            let queue = app_clone.state::<wgpu::Queue>();
                            let device = app_clone.state::<wgpu::Device>();
                            let surface = app_clone.state::<wgpu::Surface>();
                            let render_pipeline = app_clone.state::<wgpu::RenderPipeline>();

                            //////
                            // this errors out
                            // let bind_group = rx.recv().unwrap();

                            let output = surface
                                .get_current_texture()
                                .expect("Failed to acquire next swap chain texture");
                            let view = output
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder =
                                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: None,
                                });
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
                                        timestamp_writes: None,
                                        occlusion_query_set: None,
                                        depth_stencil_attachment: None,
                                    });
                                rpass.set_pipeline(&render_pipeline);
                                rpass.set_bind_group(0, &bind_group, &[]);
                                rpass.draw(0..6, 0..1);
                            }

                            queue.submit(Some(encoder.finish()));
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
                    let device = app_handle.state::<wgpu::Device>();
                    let surface = app_handle.state::<wgpu::Surface>();
                    let config = app_handle.state::<Mutex<wgpu::SurfaceConfiguration>>();

                    let mut config = config.lock().unwrap();
                    config.width = if size.width > 0 { size.width } else { 1 };
                    config.height = if size.height > 0 { size.height } else { 1 };
                    surface.configure(&device, &config)

                    // TODO: Request redraw on macos (not exposed in taurip yet).
                }
                RunEvent::MainEventsCleared => {
                    // println!("MainEventsCleared");
                }
                _ => (),
            }
        });
}
