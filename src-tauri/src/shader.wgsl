@group(0) @binding(0) var my_texture: texture_2d<f32>;
@group(0) @binding(1) var my_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Determine vertex position and texture coordinates based on the vertex index
    switch (in_vertex_index) {
        case 0u: {
            out.position = vec4<f32>(-1.0, -1.0, 0.0, 1.0); // Bottom-left
            out.tex_coords = vec2<f32>(0.0, 1.0);
        }
        case 1u: {
            out.position = vec4<f32>(1.0, -1.0, 0.0, 1.0); // Bottom-right
            out.tex_coords = vec2<f32>(1.0, 1.0);
        }
        case 2u: {
            out.position = vec4<f32>(-1.0, 1.0, 0.0, 1.0); // Top-left
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
        case 3u: {
            out.position = vec4<f32>(1.0, -1.0, 0.0, 1.0); // Bottom-right (shared with previous triangle)
            out.tex_coords = vec2<f32>(1.0, 1.0);
        }
        case 4u: {
            out.position = vec4<f32>(-1.0, 1.0, 0.0, 1.0); // Top-left (shared with previous triangle)
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
        case 5u: {
            out.position = vec4<f32>(1.0, 1.0, 0.0, 1.0); // Top-right
            out.tex_coords = vec2<f32>(1.0, 0.0);
        }
        default: {
            // Default case to handle unexpected indices
            out.position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
            out.tex_coords = vec2<f32>(0.0, 0.0);
        }
    }

    return out;
}

@fragment
fn fs_main(@location(0) in_tex_coords: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(my_texture, my_sampler, in_tex_coords);
}
