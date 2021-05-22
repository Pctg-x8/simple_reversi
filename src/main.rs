use glfw::Context;

fn main() {
    let mut glfw =
        glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to initialize glfw");
    let (mut window, events) = glfw
        .create_window(480, 480, "Simple Reversi", glfw::WindowMode::Windowed)
        .expect("Failed to create window");
    window.set_key_polling(true);
    window.make_current();
    gl::load_with(|s| glfw.get_proc_address_raw(s));
    let buffers = Buffers::new();
    let shaders = Shaders::new();

    while !window.should_close() {
        glfw.poll_events();
        for (_, e) in glfw::flush_messages(&events) {
            match e {
                _ => {}
            }
        }
        update(&buffers, &shaders);
        window.swap_buffers();
    }
}

struct BufferBindPoint(gl::types::GLenum);
impl BufferBindPoint {
    pub fn bind(&self, buf: gl::types::GLuint) -> &Self {
        unsafe {
            gl::BindBuffer(self.0, buf);
        }
        self
    }
    pub fn unbind(&self) -> &Self {
        unsafe {
            gl::BindBuffer(self.0, 0);
        }
        self
    }
    pub fn data<T>(&self, slice: &[T], usage: gl::types::GLenum) -> &Self {
        unsafe {
            gl::BufferData(
                self.0,
                (std::mem::size_of::<T>() * slice.len()) as _,
                slice.as_ptr() as _,
                usage,
            );
        }
        self
    }
}
static ARRAY_BUFFER: BufferBindPoint = BufferBindPoint(gl::ARRAY_BUFFER);

struct Buffers {
    fillrect_vb: gl::types::GLuint,
    fillrect_va: gl::types::GLuint,
}
impl Buffers {
    pub fn new() -> Self {
        const FILLRECT_VERTICES: &'static [[f32; 2]; 4] =
            &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];

        let mut fillrect_va = 0;
        let mut fillrect_vb = 0;
        unsafe {
            gl::GenBuffers(1, &mut fillrect_vb);
            ARRAY_BUFFER
                .bind(fillrect_vb)
                .data(FILLRECT_VERTICES, gl::STATIC_DRAW)
                .unbind();

            gl::GenVertexArrays(1, &mut fillrect_va);
            gl::BindVertexArray(fillrect_va);
            ARRAY_BUFFER.bind(fillrect_vb);
            gl::EnableVertexArrayAttrib(fillrect_va, 0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null(),
            );
            gl::BindVertexArray(0);
            ARRAY_BUFFER.unbind();
        }

        Buffers {
            fillrect_vb,
            fillrect_va,
        }
    }
}
impl Drop for Buffers {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.fillrect_va);
            gl::DeleteBuffers(1, &self.fillrect_vb);
        }
    }
}

struct Shader(gl::types::GLuint);
impl Shader {
    pub fn compile_file(ty: gl::types::GLenum, path: &str) -> Self {
        let code =
            std::fs::read_to_string(path).expect("Failed to load shader");
        unsafe {
            let sh = gl::CreateShader(ty);
            gl::ShaderSource(
                sh,
                1,
                &(code.as_ptr() as *const i8) as _,
                &(code.len() as _),
            );
            gl::CompileShader(sh);
            let mut compilation_succeeded_flag = 0;
            gl::GetShaderiv(
                sh,
                gl::COMPILE_STATUS,
                &mut compilation_succeeded_flag,
            );
            if compilation_succeeded_flag == gl::FALSE as _ {
                let mut infolog_length = 0;
                gl::GetShaderiv(sh, gl::INFO_LOG_LENGTH, &mut infolog_length);
                let mut infolog = vec![0u8; infolog_length as usize];
                gl::GetShaderInfoLog(
                    sh,
                    infolog_length,
                    std::ptr::null_mut(),
                    infolog.as_mut_ptr() as _,
                );
                panic!(
                    "Shader Compilation was not successful: {}",
                    std::ffi::CStr::from_bytes_with_nul_unchecked(&infolog)
                        .to_str()
                        .expect("invalid utf8 character in infolog")
                );
            }

            Shader(sh)
        }
    }
}
impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.0);
        }
    }
}
struct Program(gl::types::GLuint);
impl Program {
    pub fn link_shaders(shaders: &[&Shader]) -> Self {
        unsafe {
            let p = gl::CreateProgram();
            for sh in shaders {
                gl::AttachShader(p, sh.0);
            }
            gl::LinkProgram(p);
            let mut link_succeeded_flag = 0;
            gl::GetProgramiv(p, gl::LINK_STATUS, &mut link_succeeded_flag);
            if link_succeeded_flag == gl::FALSE as _ {
                let mut infolog_length = 0;
                gl::GetProgramiv(p, gl::INFO_LOG_LENGTH, &mut infolog_length);
                let mut infolog = vec![0u8; infolog_length as usize];
                gl::GetProgramInfoLog(
                    p,
                    infolog_length,
                    std::ptr::null_mut(),
                    infolog.as_mut_ptr() as _,
                );
                panic!(
                    "Shader Linking was not successful: {}",
                    std::ffi::CStr::from_bytes_with_nul_unchecked(&infolog)
                        .to_str()
                        .expect("invalid utf8 character in infolog")
                );
            }

            Program(p)
        }
    }
    pub fn uniform_location(
        &self,
        name: &std::ffi::CStr,
    ) -> Option<gl::types::GLint> {
        let r = unsafe { gl::GetUniformLocation(self.0, name.as_ptr()) };
        if r < 0 {
            None
        } else {
            Some(r)
        }
    }
}
impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.0);
        }
    }
}

struct Shaders {
    board_base_render: Program,
    board_base_render_scale_uniform_location: gl::types::GLint,
    board_grid_render: Program,
    board_grid_render_scale_uniform_location: gl::types::GLint,
}
impl Shaders {
    pub fn new() -> Self {
        let scaled_vsh =
            Shader::compile_file(gl::VERTEX_SHADER, "./assets/scaled.vsh");
        let board_base_fsh = Shader::compile_file(
            gl::FRAGMENT_SHADER,
            "./assets/board_base.fsh",
        );
        let board_grid_fsh = Shader::compile_file(
            gl::FRAGMENT_SHADER,
            "./assets/board_grid.fsh",
        );

        let board_base_render =
            Program::link_shaders(&[&scaled_vsh, &board_base_fsh]);
        let board_base_render_scale_uniform_location = board_base_render
            .uniform_location(unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"scale\0")
            })
            .expect("no scale uniform defined");
        let board_grid_render =
            Program::link_shaders(&[&scaled_vsh, &board_grid_fsh]);
        let board_grid_render_scale_uniform_location = board_grid_render
            .uniform_location(unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"scale\0")
            })
            .expect("no scale uniform defined");

        Shaders {
            board_base_render,
            board_base_render_scale_uniform_location,
            board_grid_render,
            board_grid_render_scale_uniform_location,
        }
    }
}

fn update(buffers: &Buffers, shaders: &Shaders) {
    unsafe {
        gl::ClearColor(0.0, 0.4, 0.8, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        gl::BindVertexArray(buffers.fillrect_va);
        gl::UseProgram(shaders.board_base_render.0);
        gl::Uniform1f(shaders.board_base_render_scale_uniform_location, 0.8);
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        gl::UseProgram(shaders.board_grid_render.0);
        gl::Uniform1f(shaders.board_grid_render_scale_uniform_location, 0.78);
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        gl::BindVertexArray(0);
    }
}
