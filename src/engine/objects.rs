// Put OpenGL Objects here

use std::{
    env,
    ffi::{CStr, CString},
    marker::PhantomData,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use gl::{
    types::{GLchar, GLenum, GLint, GLuint},
    UseProgram,
};

use image::{EncodableLayout, ImageError};

// An OpenGL Shader
pub struct Shader {
    id: GLuint,
}

impl Shader {
    pub fn from_source(source: &CStr, kind: GLenum) -> Result<Self, String> {
        let id = unsafe { gl::CreateShader(kind) };

        unsafe {
            gl::ShaderSource(id, 1, &source.as_ptr(), null());
            gl::CompileShader(id);
        }

        let mut success: GLint = 1;
        unsafe {
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
        }

        if success == 0 {
            // Error occured!
            let mut len: GLint = 0;
            unsafe { gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len) }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetShaderInfoLog(id, len, null_mut(), error.as_ptr() as *mut GLchar);
            }

            return Err(error.to_string_lossy().into_owned());
        }

        Ok(Shader { id })
    }

    pub fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

#[derive(Default)]
pub struct Program {
    id: GLuint,
}

impl Program {
    fn from_shaders(shaders: &[Shader]) -> Result<Self, String> {
        let id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(id, shader.id());
            }
        }

        unsafe {
            gl::LinkProgram(id);
        }

        let mut success: GLint = 1;
        unsafe {
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            // An error occured
            let mut len: GLint = 0;
            unsafe {
                gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(id, len, null_mut(), error.as_ptr() as *mut GLchar);
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe {
                gl::DetachShader(id, shader.id());
            }
        }

        Ok(Program { id })
    }

    pub fn set(&self) {
        unsafe {
            UseProgram(self.id);
        }
    }

    pub fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn create_program(
    vert_data: &'static str,
    frag_data: &'static str,
) -> Result<Program, &'static str> {
    let vert_shader = Shader::from_source(
        &CString::new(vert_data).unwrap(), // TODO: Load this at runtime
        gl::VERTEX_SHADER,
    )
    .unwrap();
    let frag_shader = Shader::from_source(
        &CString::new(frag_data).unwrap(), // TODO: Load this at runtime
        gl::FRAGMENT_SHADER,
    )
    .unwrap();

    let shader_program = Program::from_shaders(&[vert_shader, frag_shader]).unwrap();

    Ok(shader_program)
}

// OpenGL Vertex Buffer Object
// Contains vertex data given as input to the vertex shader
pub struct Buffer<T> {
    pub id: GLuint,
    target: GLenum,
    phantom: PhantomData<T>,
}

impl<T> Buffer<T> {
    pub fn gen(target: GLenum) -> Self {
        let mut id: GLuint = 0;
        unsafe {
            gl::GenBuffers(1, &mut id);
        }
        Buffer::<T> {
            id,
            target,
            phantom: PhantomData::<T>::default(),
        }
    }

    pub fn set_data(&self, data: &Vec<T>) {
        self.bind();
        unsafe {
            gl::BufferData(
                self.target,
                (data.len() * std::mem::size_of::<T>()) as gl::types::GLsizeiptr,
                data.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindBuffer(self.target, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindBuffer(self.target, 0);
        }
    }

    fn delete(&self) {
        unsafe {
            gl::DeleteBuffers(1, &self.id);
        }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        self.unbind();
        self.delete();
    }
}

/// OpenGL Vertex Array Object
pub struct Vao {
    pub id: GLuint,
}

impl Vao {
    pub fn gen() -> Self {
        let mut id: GLuint = 0;
        unsafe {
            gl::GenVertexArrays(1, &mut id);
        }
        Vao { id }
    }

    pub fn set(&self, loc: u32) {
        self.bind(loc);
        self.setup(loc);
    }

    pub fn enable(&self, loc: u32) {
        unsafe {
            gl::EnableVertexAttribArray(loc);
        }
        self.setup(loc);
    }

    fn bind(&self, loc: u32) {
        unsafe {
            gl::EnableVertexAttribArray(loc);
            gl::BindVertexArray(self.id);
        }
    }

    fn setup(&self, loc: u32) {
        unsafe {
            gl::VertexAttribPointer(
                loc,
                3,
                gl::FLOAT,
                gl::FALSE,
                (3 * std::mem::size_of::<f32>()) as GLint,
                null(),
            );
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindVertexArray(0);
        }
    }

    fn delete(&self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.id);
        }
    }
}

impl Drop for Vao {
    fn drop(&mut self) {
        self.unbind();
        self.delete();
    }
}

pub struct Uniform {
    pub id: GLint,
}

impl Uniform {
    pub fn new(program: u32, name: &str) -> Result<Self, &'static str> {
        let cname: CString = CString::new(name).expect("CString::new failed");
        let location: GLint = unsafe { gl::GetUniformLocation(program, cname.as_ptr()) };
        if location == -1 {
            return Err("Couldn't get a uniform location");
        }
        Ok(Uniform { id: location })
    }
}

#[derive(Clone)]
pub struct Texture {
    pub id: GLuint,
}

impl Texture {
    pub fn new() -> Self {
        let mut id: GLuint = 0;
        unsafe { gl::GenTextures(1, &mut id) }
        Self { id }
    }

    pub fn from_png(texture_filename: &'static str) -> Self {
        let texture = Texture::new();
        const RES_PATH: &str = "C:\\Users\\Joseph\\git\\survival\\res";
        let res_path = Path::new(RES_PATH);
        let mut path = PathBuf::from(res_path);
        path.push(texture_filename);
        texture.load(&path).unwrap();
        texture
    }

    pub fn from_surface(surface: sdl2::surface::Surface) -> Self {
        let texture = Texture::new();
        unsafe {
            texture.bind();

            let width = surface.width() as i32;
            let height = surface.height() as i32;
            let format = match surface.pixel_format_enum() {
                sdl2::pixels::PixelFormatEnum::RGB24 => gl::RGB,
                sdl2::pixels::PixelFormatEnum::RGBA32 => gl::RGBA,
                sdl2::pixels::PixelFormatEnum::ARGB8888 => gl::BGRA,
                x => panic!("Lol! {:?}", x),
            };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::SRGB as i32,
                width,
                height,
                0,
                format,
                gl::UNSIGNED_BYTE,
                surface.without_lock().unwrap().as_ptr() as *const std::ffi::c_void,
            );

            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        }
        texture
    }

    pub fn bind(&self) {
        unsafe { gl::BindTexture(gl::TEXTURE_2D, self.id) }
    }

    pub fn load(&self, path: &Path) -> Result<(), ImageError> {
        self.bind();

        let img = image::open(path)?.into_rgba8();
        unsafe {
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_EDGE as GLint,
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::SRGB as i32,
                img.width() as i32,
                img.height() as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                img.as_bytes().as_ptr() as *const _,
            );
            gl::GenerateMipmap(gl::TEXTURE_2D);
        }
        Ok(())
    }

    pub fn load_depth_buffer(&self, width: i32, height: i32) {
        self.bind();

        unsafe {
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::DEPTH_COMPONENT as GLint,
                width,
                height,
                0,
                gl::DEPTH_COMPONENT,
                gl::FLOAT,
                std::ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_S,
                gl::CLAMP_TO_BORDER as GLint,
            );
            gl::TexParameteri(
                gl::TEXTURE_2D,
                gl::TEXTURE_WRAP_T,
                gl::CLAMP_TO_BORDER as GLint,
            );
            let border_color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
            gl::TexParameterfv(
                gl::TEXTURE_2D,
                gl::TEXTURE_BORDER_COLOR,
                border_color.as_ptr(),
            );
        }
    }

    pub fn post_bind(&self) {
        unsafe {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::TEXTURE_2D,
                self.id,
                0,
            );
            gl::DrawBuffer(gl::NONE);
            gl::ReadBuffer(gl::NONE);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Framebuffer is not complete!");
            }
        };
    }

    pub fn activate(&self, unit: GLuint) {
        unsafe {
            gl::ActiveTexture(unit);
            self.bind();
        }
    }

    pub fn associate_uniform(&self, program_id: u32, unit: GLint, uniform_name: &str) {
        unsafe {
            let uniform = CString::new(uniform_name).unwrap();
            gl::Uniform1i(gl::GetUniformLocation(program_id, uniform.as_ptr()), unit)
        }
    }

    pub fn get_dimensions(&self) -> Option<(i32, i32)> {
        let mut width: GLint = 0;
        let mut height: GLint = 0;

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
            gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_WIDTH, &mut width);
            gl::GetTexLevelParameteriv(gl::TEXTURE_2D, 0, gl::TEXTURE_HEIGHT, &mut height);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        if width > 0 && height > 0 {
            Some((width, height))
        } else {
            None
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, [self.id].as_ptr());
        }
    }
}
impl Default for Texture {
    fn default() -> Self {
        Self { id: 0 }
    }
}

pub struct Fbo {
    pub id: GLuint,
}

impl Fbo {
    pub fn new() -> Self {
        let mut id: GLuint = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut id);
        }
        Self { id }
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id);
        }
    }

    pub fn unbind(&self) {
        unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0) }
    }
}

impl Default for Fbo {
    fn default() -> Self {
        Self { id: 0 }
    }
}
