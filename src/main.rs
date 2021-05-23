use std::{convert::TryFrom, ffi::c_void};

use glfw::Context;
use rusty_v8 as v8;

fn main() {
    let mut glfw =
        glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to initialize glfw");
    glfw.window_hint(glfw::WindowHint::Resizable(false));
    if cfg!(target_os = "macos") {
        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(2));
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));
    }
    let (mut window, events) = glfw
        .create_window(480, 480, "Simple Reversi", glfw::WindowMode::Windowed)
        .expect("Failed to create window");
    window.set_mouse_button_polling(true);
    window.set_cursor_pos_polling(true);
    window.make_current();
    gl::load_with(|s| glfw.get_proc_address_raw(s));
    println!("Using OpenGL {}", unsafe {
        std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as _)
            .to_str()
            .expect("invalid string")
    });
    let buffers = Buffers::new();
    let shaders = Shaders::new();
    let mut se = ScriptEngine::new();
    let code = std::fs::read_to_string("./scripts/index.js")
        .expect("Failed to load script");
    se.execute_code(&code);

    let timer = std::time::Instant::now();
    while !window.should_close() {
        glfw.poll_events();
        for (_, e) in glfw::flush_messages(&events) {
            match e {
                glfw::WindowEvent::MouseButton(
                    glfw::MouseButton::Button1,
                    glfw::Action::Press,
                    _,
                ) => {
                    se.set_button_pressing_state(true);
                }
                glfw::WindowEvent::MouseButton(
                    glfw::MouseButton::Button1,
                    glfw::Action::Release,
                    _,
                ) => {
                    se.set_button_pressing_state(false);
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    se.set_cursor_pos(x, y);
                }
                _ => {}
            }
        }

        let elapsed = timer.elapsed();
        se.set_current_time(elapsed);

        se.next_frame();
        if let Some(bv) = se
            .iso
            .get_slot_mut::<IsoState>()
            .expect("no state bound")
            .new_border_state_buffer
            .take()
        {
            let mut scope =
                v8::HandleScope::with_context(&mut se.iso, &se.context);
            let bv = v8::Local::new(&mut scope, bv);
            let bs = bv.get_backing_store();
            UNIFORM_BUFFER
                .bind(buffers.board_state_buffer)
                .subdata_ptr(bs.data(), bs.byte_length() as _, 0)
                .unbind();
        }
        update(&buffers, &shaders, elapsed.as_nanos() as f64 / 1_000_000.0);
        window.swap_buffers();
    }
}

fn update(buffers: &Buffers, shaders: &Shaders, time_ms: f64) {
    const STONE_RENDER_WORLD_TRANSFORM: &'static [f32; 4 * 4] = &[
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        2.0,
        0.0,
        0.0,
        6.0,
        1.0 + 6.0 * 2.0,
    ];

    unsafe {
        gl::ClearColor(0.0, 0.4, 0.8, 1.0);
        gl::ClearDepth(1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

        gl::BindVertexArray(buffers.fillrect_va);
        gl::UseProgram(shaders.board_base_render.0);
        gl::Uniform1f(shaders.board_base_render_scale_uniform_location, 0.8);
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        gl::UseProgram(shaders.board_grid_render.0);
        gl::Uniform1f(shaders.board_grid_render_scale_uniform_location, 0.78);
        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
        gl::Enable(gl::DEPTH_TEST);
        gl::UseProgram(shaders.stone_render.0);
        gl::UniformMatrix4fv(
            shaders.stone_render_wt_uniform_location,
            1,
            gl::FALSE,
            STONE_RENDER_WORLD_TRANSFORM as _,
        );
        gl::Uniform1f(shaders.stone_render_time_uniform_location, time_ms as _);
        gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, buffers.board_state_buffer);
        gl::BindVertexArray(buffers.stone_va);
        gl::DrawElementsInstanced(
            gl::TRIANGLES,
            buffers.stone_index_count as _,
            gl::UNSIGNED_SHORT,
            std::ptr::null(),
            8 * 8,
        );
        UNIFORM_BUFFER.unbind();
        gl::Disable(gl::DEPTH_TEST);
        gl::BindVertexArray(0);
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
    pub fn alloc(
        &self,
        size: gl::types::GLsizeiptr,
        usage: gl::types::GLenum,
    ) -> &Self {
        unsafe {
            gl::BufferData(self.0, size, std::ptr::null(), usage);
        }
        self
    }
    pub fn subdata_ptr(
        &self,
        ptr: *mut c_void,
        size: gl::types::GLsizeiptr,
        offset: gl::types::GLintptr,
    ) -> &Self {
        unsafe {
            gl::BufferSubData(self.0, offset, size, ptr);
        }
        self
    }
}
static ARRAY_BUFFER: BufferBindPoint = BufferBindPoint(gl::ARRAY_BUFFER);
static ELEMENT_ARRAY_BUFFER: BufferBindPoint =
    BufferBindPoint(gl::ELEMENT_ARRAY_BUFFER);
static UNIFORM_BUFFER: BufferBindPoint = BufferBindPoint(gl::UNIFORM_BUFFER);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CellState {
    pub state_flags: u32,
    pub flip_start_time: f32,
}

struct Buffers {
    fillrect_vb: gl::types::GLuint,
    fillrect_va: gl::types::GLuint,
    stone_vb: gl::types::GLuint,
    stone_index_vb: gl::types::GLuint,
    stone_va: gl::types::GLuint,
    stone_index_count: usize,
    board_state_buffer: gl::types::GLuint,
}
impl Buffers {
    pub fn new() -> Self {
        const FILLRECT_VERTICES: &'static [[f32; 2]; 4] =
            &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
        const STONE_SURFACE_VERTEX_COUNT: usize = 36;
        let stone_surface_vertex_points =
            (0..STONE_SURFACE_VERTEX_COUNT).map(|x| {
                (x as f32 * std::f32::consts::TAU
                    / STONE_SURFACE_VERTEX_COUNT as f32)
                    .sin_cos()
            });
        let stone_vertices: Vec<_> = stone_surface_vertex_points
            .flat_map(|(s, c)| vec![[s, c, 0.0, 1.0], [s, c, 1.0, 1.0]])
            .collect();
        // 0, 1, 2, 2,  1, 3, 3,  2, 4, 4,  3, 5...
        // _, 1, 1, 0, -1, 2, 0, -1, 2, 0, -1, 2...
        let stone_side_indices = [1, 1]
            .iter()
            .copied()
            .chain(
                std::iter::repeat(&[0, -1, 2])
                    .take(STONE_SURFACE_VERTEX_COUNT * 2 - 1)
                    .flat_map(|x| x.iter().copied()),
            )
            .chain([0].iter().copied())
            .scan(0, |st, x| {
                let ost = *st;
                *st += x;
                // never negative
                Some((ost as u16) % (STONE_SURFACE_VERTEX_COUNT as u16 * 2))
            });
        let stone_surface_indices: Vec<_> = (1..STONE_SURFACE_VERTEX_COUNT - 1)
            .flat_map(|x| vec![0, x as u16, x as u16 + 1])
            .collect();
        let stone_indices: Vec<u16> = stone_side_indices
            .chain(stone_surface_indices.iter().map(|&x| x * 2))
            .chain(stone_surface_indices.iter().map(|&x| x * 2 + 1))
            .collect();

        let mut vbs = [0, 0, 0, 0];
        let mut vas = [0, 0];
        unsafe {
            gl::GenBuffers(vbs.len() as _, vbs.as_mut_ptr());
            gl::GenVertexArrays(vas.len() as _, vas.as_mut_ptr());
        }
        let [fillrect_vb, stone_vb, stone_index_vb, board_state_buffer] = vbs;
        let [fillrect_va, stone_va] = vas;
        unsafe {
            ARRAY_BUFFER
                .bind(fillrect_vb)
                .data(FILLRECT_VERTICES, gl::STATIC_DRAW)
                .bind(stone_vb)
                .data(&stone_vertices, gl::STATIC_DRAW)
                .unbind();
            ELEMENT_ARRAY_BUFFER
                .bind(stone_index_vb)
                .data(&stone_indices, gl::STATIC_DRAW)
                .unbind();
            UNIFORM_BUFFER
                .bind(board_state_buffer)
                // Note: std140 layout uses 16 byte stride for arrays
                .alloc(8 * 8 * 16, gl::DYNAMIC_DRAW)
                .unbind();

            gl::BindVertexArray(fillrect_va);
            ARRAY_BUFFER.bind(fillrect_vb);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null(),
            );
            gl::BindVertexArray(stone_va);
            ARRAY_BUFFER.bind(stone_vb);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                4,
                gl::FLOAT,
                gl::FALSE,
                0,
                std::ptr::null(),
            );
            ELEMENT_ARRAY_BUFFER.bind(stone_index_vb);
            gl::BindVertexArray(0);
            ELEMENT_ARRAY_BUFFER.unbind();
            ARRAY_BUFFER.unbind();
        }

        Buffers {
            fillrect_vb,
            fillrect_va,
            stone_vb,
            stone_index_vb,
            stone_va,
            stone_index_count: stone_indices.len(),
            board_state_buffer,
        }
    }
}
impl Drop for Buffers {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(
                4,
                [
                    self.fillrect_vb,
                    self.stone_vb,
                    self.stone_index_vb,
                    self.board_state_buffer,
                ]
                .as_ptr(),
            );
            gl::DeleteBuffers(2, [self.fillrect_va, self.stone_va].as_ptr());
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
    pub fn uniform_block_location(
        &self,
        name: &std::ffi::CStr,
    ) -> gl::types::GLuint {
        unsafe { gl::GetUniformBlockIndex(self.0, name.as_ptr()) }
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
    stone_render: Program,
    stone_render_wt_uniform_location: gl::types::GLint,
    stone_render_time_uniform_location: gl::types::GLint,
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
        let stone_vsh =
            Shader::compile_file(gl::VERTEX_SHADER, "./assets/stone.vsh");
        let stone_fsh =
            Shader::compile_file(gl::FRAGMENT_SHADER, "./assets/stone.fsh");

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
        let stone_render = Program::link_shaders(&[&stone_vsh, &stone_fsh]);
        let stone_render_wt_uniform_location = stone_render
            .uniform_location(unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(
                    b"world_transform\0",
                )
            })
            .expect("no world transform uniform defined");
        let stone_render_time_uniform_location = stone_render
            .uniform_location(unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"time_ms\0")
            })
            .expect("no time uniform defined");
        let board_state_uniform_block_location = stone_render
            .uniform_block_location(unsafe {
                std::ffi::CStr::from_bytes_with_nul_unchecked(b"BoardState\0")
            });
        unsafe {
            gl::UniformBlockBinding(
                stone_render.0,
                board_state_uniform_block_location,
                0,
            );
        }

        Shaders {
            board_base_render,
            board_base_render_scale_uniform_location,
            board_grid_render,
            board_grid_render_scale_uniform_location,
            stone_render,
            stone_render_wt_uniform_location,
            stone_render_time_uniform_location,
        }
    }
}

pub struct IsoState {
    pub next_frame_callbacks: Vec<v8::Global<v8::Function>>,
    pub cursor_pos: (f64, f64),
    pub button_pressing: bool,
    pub new_border_state_buffer: Option<v8::Global<v8::ArrayBuffer>>,
    pub current_time_ms: f64,
}
impl IsoState {
    pub fn new() -> Self {
        Self {
            next_frame_callbacks: Vec::new(),
            cursor_pos: (0.0, 0.0),
            button_pressing: false,
            new_border_state_buffer: None,
            current_time_ms: 0.0,
        }
    }
}

fn request_next_frame(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let f = match v8::Local::<v8::Function>::try_from(args.get(0)) {
        Ok(f) => v8::Global::new(scope, f),
        Err(e) => {
            let msg = v8::String::new(scope, &e.to_string())
                .expect("Failed to create error message");
            let err = v8::Exception::type_error(scope, msg);
            scope.throw_exception(err);
            return;
        }
    };

    scope
        .get_slot_mut::<IsoState>()
        .expect("no state bound?")
        .next_frame_callbacks
        .push(f);
}
fn is_button_pressing(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let pressing = scope
        .get_slot::<IsoState>()
        .expect("no state bound")
        .button_pressing;
    let v = v8::Boolean::new(scope, pressing);
    rv.set(v.into());
}
fn cursor_pos(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let (cx, cy) = scope
        .get_slot::<IsoState>()
        .expect("no state bound")
        .cursor_pos;
    let vx = v8::Number::new(scope, cx);
    let vy = v8::Number::new(scope, cy);
    let va = v8::Array::new_with_elements(scope, &[vx.into(), vy.into()]);
    rv.set(va.into());
}
fn set_board_state_buffer(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let v = match v8::Local::<v8::ArrayBuffer>::try_from(args.get(0)) {
        Ok(v) => v8::Global::new(scope, v),
        Err(e) => {
            let msg = v8::String::new(scope, &e.to_string())
                .expect("Failed to create error message");
            let err = v8::Exception::type_error(scope, msg);
            scope.throw_exception(err);
            return;
        }
    };

    scope
        .get_slot_mut::<IsoState>()
        .expect("no state bound")
        .new_border_state_buffer = Some(v);
}
fn current_time_ms(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let t = scope
        .get_slot::<IsoState>()
        .expect("no state bound")
        .current_time_ms;
    let v = v8::Number::new(scope, t);
    rv.set(v.into());
}

pub struct ScriptEngine {
    // Note: Inspectors must be destroyed before isolate destruction
    _inspector: v8::UniqueRef<v8::inspector::V8Inspector>,
    _inspector_client: Box<ScriptInspectorClient>,
    iso: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
}
impl ScriptEngine {
    pub fn new() -> Self {
        let platform = v8::new_default_platform().unwrap();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();

        let mut iso = v8::Isolate::new(v8::CreateParams::default());
        iso.set_slot(IsoState::new());
        let mut inspector_client = Box::new(ScriptInspectorClient::new());
        let mut inspector = v8::inspector::V8Inspector::create(
            &mut iso,
            &mut *inspector_client,
        );
        let context = {
            let mut scope = v8::HandleScope::new(&mut iso);
            let context = v8::Context::new(&mut scope);
            let mut scope = v8::ContextScope::new(&mut scope, context);

            inspector.context_created(
                context,
                1,
                v8::inspector::StringView::from(&b"ScriptInspector"[..]),
            );

            // register global exposures
            let global = context.global(&mut scope);
            let name = v8::String::new(&mut scope, "requestNextFrame")
                .expect("Failed to create function name object");
            let func =
                v8::FunctionTemplate::new(&mut scope, request_next_frame)
                    .get_function(&mut scope)
                    .expect("Failed to get requestNextFrame function");
            global.set(&mut scope, name.into(), func.into());
            let name = v8::String::new(&mut scope, "isButtonPressing")
                .expect("Failed to create function name object");
            let func =
                v8::FunctionTemplate::new(&mut scope, is_button_pressing)
                    .get_function(&mut scope)
                    .expect("Failed to get isButtonPressing function");
            global.set(&mut scope, name.into(), func.into());
            let name = v8::String::new(&mut scope, "cursorPos")
                .expect("Failed to create function name object");
            let func = v8::FunctionTemplate::new(&mut scope, cursor_pos)
                .get_function(&mut scope)
                .expect("Failed to get cursorPos function");
            global.set(&mut scope, name.into(), func.into());
            let name = v8::String::new(&mut scope, "setBoardStateBuffer")
                .expect("Failed to create function name object");
            let func =
                v8::FunctionTemplate::new(&mut scope, set_board_state_buffer)
                    .get_function(&mut scope)
                    .expect("Failed to get setBoardStateBuffer function");
            global.set(&mut scope, name.into(), func.into());
            let name = v8::String::new(&mut scope, "currentTimeMs")
                .expect("Failed to create function name object");
            let func = v8::FunctionTemplate::new(&mut scope, current_time_ms)
                .get_function(&mut scope)
                .expect("Failed to get currentTimeMs function");
            global.set(&mut scope, name.into(), func.into());

            v8::Global::new(&mut scope, context)
        };

        ScriptEngine {
            context,
            iso,
            _inspector: inspector,
            _inspector_client: inspector_client,
        }
    }

    pub fn set_cursor_pos(&mut self, x: f64, y: f64) {
        self.iso
            .get_slot_mut::<IsoState>()
            .expect("no state bound")
            .cursor_pos = (x, y);
    }
    pub fn set_button_pressing_state(&mut self, pressing: bool) {
        self.iso
            .get_slot_mut::<IsoState>()
            .expect("no state bound")
            .button_pressing = pressing;
    }
    pub fn set_current_time(&mut self, t: std::time::Duration) {
        self.iso
            .get_slot_mut::<IsoState>()
            .expect("no state bound")
            .current_time_ms = t.as_nanos() as f64 / 1_000_000.0;
    }

    pub fn execute_code(&mut self, code: &str) {
        let mut scope =
            v8::HandleScope::with_context(&mut self.iso, &self.context);
        let code = v8::String::new(&mut scope, code)
            .expect("Failed to allocate string");
        let mut tc = v8::TryCatch::new(&mut scope);
        let script = v8::Script::compile(&mut tc, code, None);
        if let Some(e) = tc.exception() {
            panic!(
                "Script compilation failed! {}",
                e.to_rust_string_lossy(&mut tc)
            );
        }
        let _res = script.expect("Script compilation failed?").run(&mut tc);
        if let Some(e) = tc.exception() {
            panic!(
                "Script terminated with unhandled exception: {}",
                e.to_rust_string_lossy(&mut tc)
            );
        }
    }

    pub fn next_frame(&mut self) {
        let callbacks = std::mem::replace(
            &mut self
                .iso
                .get_slot_mut::<IsoState>()
                .expect("no state bound")
                .next_frame_callbacks,
            Vec::new(),
        );

        let mut scope =
            v8::HandleScope::with_context(&mut self.iso, &self.context);
        let global = self.context.get(&mut scope).global(&mut scope);

        for cb in callbacks {
            cb.get(&mut scope)
                .call(&mut scope, global.into(), &[])
                .expect("Failed to callback to next frame");
        }
    }
}

pub struct ScriptInspectorClient(v8::inspector::V8InspectorClientBase);
impl ScriptInspectorClient {
    pub fn new() -> Self {
        Self(v8::inspector::V8InspectorClientBase::new::<Self>())
    }
}
impl v8::inspector::V8InspectorClientImpl for ScriptInspectorClient {
    fn base(&self) -> &v8::inspector::V8InspectorClientBase {
        &self.0
    }
    fn base_mut(&mut self) -> &mut v8::inspector::V8InspectorClientBase {
        &mut self.0
    }

    fn console_api_message(
        &mut self,
        _context_group_id: i32,
        _level: i32,
        message: &v8::inspector::StringView,
        url: &v8::inspector::StringView,
        line_number: u32,
        _column_number: u32,
        _stack_trace: &mut v8::inspector::V8StackTrace,
    ) {
        println!("ConsoleLog from {}:{}>{}", url, line_number, message);
    }
}
