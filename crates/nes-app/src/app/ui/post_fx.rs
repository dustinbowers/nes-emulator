use std::sync::Arc;
use eframe::glow;
use eframe::glow::HasContext;

#[cfg(target_arch = "wasm32")]
use crate::app::ui::shaders::{FS_300_ES, VS_300_ES};

#[cfg(not(target_arch = "wasm32"))]
use crate::app::ui::shaders::{FS_330, VS_330};

pub struct PostFx {
    gl: Arc<glow::Context>,
    tex: glow::NativeTexture,
    program: glow::NativeProgram,
    vao: glow::NativeVertexArray,
    vbo: glow::NativeBuffer,
    u_src_size: Option<glow::NativeUniformLocation>,
    u_time: Option<glow::NativeUniformLocation>,
}

impl PostFx {
    pub unsafe fn new(gl: Arc<glow::Context>) -> Self {
        // texture (256x240 RGBA8)
        let tex = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

        // allocate storage
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            256,
            240,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            eframe::glow::PixelUnpackData::Slice(None),
        );

        // shader program
        let (vs_src, fs_src) = shader_sources();
        let program = compile_program(&gl, vs_src, fs_src);

        // uniforms
        gl.use_program(Some(program));
        let u_src_size = gl.get_uniform_location(program, "u_src_size");
        let u_time = gl.get_uniform_location(program, "u_time");
        let u_tex = gl.get_uniform_location(program, "u_tex");

        // set sampler unit once
        if let Some(u_tex) = &u_tex {
            gl.uniform_1_i32(Some(u_tex), 0);
        }

        // quad geometry - vec2 pos, vec2 uv (4 vertices, triangle strip)
        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

        // positions + UVs will be updated per-draw
        gl.buffer_data_size(glow::ARRAY_BUFFER, 4 * 4 * 4, glow::DYNAMIC_DRAW);

        let stride = 4 * 4;
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, (2 * 4) as i32);

        gl.bind_vertex_array(None);

        Self { gl, tex, program, vao, vbo, u_src_size, u_time }
    }

    pub unsafe fn upload_frame(&self, rgba: &[u8]) {
        debug_assert_eq!(rgba.len(), 256 * 240 * 4);
        self.gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));
        self.gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            256,
            240,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Option::from(rgba)),
        );
    }

    pub unsafe fn paint(
        &self,
        gl: &glow::Context,
        info: egui::PaintCallbackInfo,
        time: f32,
    ) {
        let vp = info.viewport_in_pixels();
        gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);

        gl.use_program(Some(self.program));

        // gl.uniform_2_f32(Some(&self.u_src_size), 256.0, 240.0);
        // gl.uniform_1_f32(Some(&self.u_time), time);
        if let Some(u) = &self.u_src_size {
            gl.uniform_2_f32(Some(u), 256.0, 240.0);
        }
        if let Some(u) = &self.u_time {
            gl.uniform_1_f32(Some(u), time);
        }


        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.tex));

        // Fullscreen quad in NDC; viewport handles placement/scaling
        let verts: [f32; 16] = [
            -1.0, -1.0, 0.0, 1.0,
            1.0, -1.0, 1.0, 1.0,
            -1.0,  1.0, 0.0, 0.0,
            1.0,  1.0, 1.0, 0.0,
        ];

        gl.bind_vertex_array(Some(self.vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, bytemuck::cast_slice(&verts));

        gl.disable(glow::DEPTH_TEST);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);

        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

        gl.bind_vertex_array(None);
        gl.use_program(None);
    }
}


fn shader_sources() -> (&'static str, &'static str) {
    #[cfg(target_arch = "wasm32")]
    {
        (VS_300_ES, FS_300_ES)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        (VS_330, FS_330)
    }
}


unsafe fn compile_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> glow::NativeProgram {
    let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
    gl.shader_source(vs, vs_src);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        panic!("Vertex shader compile error:\n{}", gl.get_shader_info_log(vs));
    }

    let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
    gl.shader_source(fs, fs_src);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        panic!("Fragment shader compile error:\n{}", gl.get_shader_info_log(fs));
    }

    let program = gl.create_program().unwrap();
    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("Program link error:\n{}", gl.get_program_info_log(program));
    }

    gl.delete_shader(vs);
    gl.delete_shader(fs);
    program
}
