// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nokhwa::pixel_format::RgbAFormat;
use nokhwa::utils::{RequestedFormat, RequestedFormatType};
use nokhwa::{native_api_backend, query, Camera};
use std::{borrow::Cow, sync::Mutex};
use tauri::{async_runtime, Manager, RunEvent, WindowEvent};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the main window dimensions
            let window = app.get_webview_window("main").unwrap();
            let size = window.inner_size()?;

            // Create a WGPU instance, surface and adapter
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

            // Create shader that accepts a texture and sampler
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                    r#"
            @group(0) @binding(0) var my_texture: texture_2d<f32>;
            @group(0) @binding(1) var my_sampler: sampler;
            
            @vertex
            fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(in_vertex_index) - 1);
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
                return vec4<f32>(x, y, 0.0, 1.0);
            }
            
            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return textureSample(my_texture, my_sampler, vec2<f32>(0.5, 0.5));
            }
            "#,
                )),
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: None,
            });
            

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
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
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
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
            
            app.manage(bind_group_layout);
            app.manage(surface);
            app.manage(render_pipeline);
            app.manage(device);
            app.manage(queue);
            app.manage(Mutex::new(config));
            app.manage(sampler);

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
                        let mut camera = create_camera();
                        camera.open_stream().expect("Could not open stream");

                        // std::thread::sleep(std::time::Duration::from_secs(3));

                        for i in 0..100 {
                            let frame = camera.frame().expect("Could not get frame");
                            let device = app_clone.state::<wgpu::Device>();
                            let queue = app_clone.state::<wgpu::Queue>();
                        
                            let frame = frame.decode_image::<RgbAFormat>().expect("Could not decode frame");
                        
                            let texture_size = wgpu::Extent3d {
                                width: frame.width(),
                                height: frame.height(),
                                depth_or_array_layers: 1,
                            };
                        
                            let texture = device.create_texture(&wgpu::TextureDescriptor {
                                label: None,
                                size: texture_size,
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                                view_formats: &[],
                            });
                        
                            queue.write_texture(
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
                        
                            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
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
                        
                            let test = app_clone.manage(Some(bind_group));
                            println!("{:?}", test);
                        
                            println!("Frame {i}");                        
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

                    let bind_group = app_handle.state::<wgpu::BindGroup>();

                    println!("Drawing");

                    let surface = app_handle.state::<wgpu::Surface>();
                    let render_pipeline = app_handle.state::<wgpu::RenderPipeline>();
                    let device = app_handle.state::<wgpu::Device>();
                    let queue = app_handle.state::<wgpu::Queue>();

                    // this errors out
                   

                    let output = surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");
                    let view = output
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
                                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.set_bind_group(0, &bind_group, &[]);
                        rpass.draw(0..3, 0..1);
                    }

                    queue.submit(Some(encoder.finish()));
                    output.present();
                }
                _ => (),
            }
        });
}

fn create_camera() -> Camera {
    let backend = native_api_backend().expect("Could not get backend");
    let devices = query(backend).expect("Could not query backend");
    let device = devices.first().expect("No devices found");

    let format = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::AbsoluteHighestResolution);

    Camera::new(device.index().to_owned(), format).expect("Could not create camera")
}
