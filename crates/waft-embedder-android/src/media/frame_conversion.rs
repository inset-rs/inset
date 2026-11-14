//! A camera's picture in RGBA: the camera hands out its pictures in its own format, `PRIVATE`,
//! YUV the compositor shows and only OpenGL ES samples, through an external texture; Waft's frames
//! are RGBA8 hardware buffers, which Vulkan imports and the app's GPU samples, as Apple's are BGRA
//! pixel buffers AVFoundation converts into. So the host draws each picture a reader is to have
//! into a new RGBA8 `AHardwareBuffer` with OpenGL ES, as Android's own `SurfaceTexture` draws a
//! camera's pictures: both buffers made EGL images (`EGL_ANDROID_get_native_client_buffer`), the
//! picture sampled through `samplerExternalOES`, which converts its colours, and drawn row for row,
//! unturned, onto the buffer as the colour target of a framebuffer.
//!
//! One thread draws for the process, with one context current on it; a conversion asked from any
//! thread waits for it, and its answer is a buffer the GPU has finished drawing (`glFinish`), as
//! Waft's frames are handed out with no fence. A frame is its own buffer, freed once nothing holds
//! it: no pool hands it out again.
//!
//! The invariant: every GL call is made on the drawing thread, and every EGL image made for a
//! conversion is destroyed before its answer is given.

// EGL's and OpenGL ES's C APIs are `unsafe` to call; this module writes it for its callers.
#![allow(unsafe_code)]

use std::ffi::{CStr, c_char, c_void};
use std::ptr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock, PoisonError};

use ndk::hardware_buffer::{
    HardwareBuffer, HardwareBufferDesc, HardwareBufferRef, HardwareBufferUsage,
};
use ndk::hardware_buffer_format::HardwareBufferFormat;

use crate::hardware_buffers::AndroidHardwareBuffer;

type EglDisplay = *mut c_void;
type EglConfig = *mut c_void;
type EglContext = *mut c_void;
type EglSurface = *mut c_void;
type EglImage = *mut c_void;
type EglClientBuffer = *mut c_void;

#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetDisplay(display: *mut c_void) -> EglDisplay;
    fn eglInitialize(display: EglDisplay, major: *mut i32, minor: *mut i32) -> u32;
    fn eglBindAPI(api: u32) -> u32;
    fn eglChooseConfig(
        display: EglDisplay,
        attributes: *const i32,
        configs: *mut EglConfig,
        size: i32,
        count: *mut i32,
    ) -> u32;
    fn eglCreateContext(
        display: EglDisplay,
        config: EglConfig,
        share: EglContext,
        attributes: *const i32,
    ) -> EglContext;
    fn eglCreatePbufferSurface(
        display: EglDisplay,
        config: EglConfig,
        attributes: *const i32,
    ) -> EglSurface;
    fn eglMakeCurrent(
        display: EglDisplay,
        draw: EglSurface,
        read: EglSurface,
        context: EglContext,
    ) -> u32;
    fn eglGetError() -> i32;
    fn eglGetProcAddress(name: *const c_char) -> *mut c_void;
}

#[link(name = "GLESv2")]
unsafe extern "C" {
    fn glCreateShader(kind: u32) -> u32;
    fn glShaderSource(shader: u32, count: i32, sources: *const *const c_char, lengths: *const i32);
    fn glCompileShader(shader: u32);
    fn glGetShaderiv(shader: u32, name: u32, value: *mut i32);
    fn glCreateProgram() -> u32;
    fn glAttachShader(program: u32, shader: u32);
    fn glBindAttribLocation(program: u32, index: u32, name: *const c_char);
    fn glLinkProgram(program: u32);
    fn glGetProgramiv(program: u32, name: u32, value: *mut i32);
    fn glUseProgram(program: u32);
    fn glGetUniformLocation(program: u32, name: *const c_char) -> i32;
    fn glUniform1i(location: i32, value: i32);
    fn glGenTextures(count: i32, textures: *mut u32);
    fn glDeleteTextures(count: i32, textures: *const u32);
    fn glBindTexture(target: u32, texture: u32);
    fn glActiveTexture(unit: u32);
    fn glTexParameteri(target: u32, name: u32, value: i32);
    fn glGenFramebuffers(count: i32, framebuffers: *mut u32);
    fn glBindFramebuffer(target: u32, framebuffer: u32);
    fn glFramebufferTexture2D(
        target: u32,
        attachment: u32,
        texture_target: u32,
        texture: u32,
        level: i32,
    );
    fn glCheckFramebufferStatus(target: u32) -> u32;
    fn glViewport(x: i32, y: i32, width: i32, height: i32);
    fn glEnableVertexAttribArray(index: u32);
    fn glVertexAttribPointer(
        index: u32,
        size: i32,
        kind: u32,
        normalized: u8,
        stride: i32,
        pointer: *const c_void,
    );
    fn glDrawArrays(mode: u32, first: i32, count: i32);
    fn glFinish();
}

type GetNativeClientBuffer = unsafe extern "C" fn(buffer: *const c_void) -> EglClientBuffer;
type CreateImage = unsafe extern "C" fn(
    display: EglDisplay,
    context: EglContext,
    target: u32,
    buffer: EglClientBuffer,
    attributes: *const i32,
) -> EglImage;
type DestroyImage = unsafe extern "C" fn(display: EglDisplay, image: EglImage) -> u32;
type ImageTargetTexture = unsafe extern "C" fn(target: u32, image: EglImage);

const EGL_DEFAULT_DISPLAY: *mut c_void = ptr::null_mut();
const EGL_NO_CONTEXT: EglContext = ptr::null_mut();
const EGL_NONE: i32 = 0x3038;
const EGL_TRUE: i32 = 1;
const EGL_OPENGL_ES_API: u32 = 0x30A0;
const EGL_RENDERABLE_TYPE: i32 = 0x3040;
const EGL_OPENGL_ES2_BIT: i32 = 0x0004;
const EGL_SURFACE_TYPE: i32 = 0x3033;
const EGL_PBUFFER_BIT: i32 = 0x0001;
const EGL_RED_SIZE: i32 = 0x3024;
const EGL_GREEN_SIZE: i32 = 0x3023;
const EGL_BLUE_SIZE: i32 = 0x3022;
const EGL_ALPHA_SIZE: i32 = 0x3021;
const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;
const EGL_NATIVE_BUFFER_ANDROID: u32 = 0x3140;
const EGL_IMAGE_PRESERVED_KHR: i32 = 0x30D2;

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_EXTERNAL_OES: u32 = 0x8D65;
const GL_TEXTURE0: u32 = 0x84C0;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: i32 = 0x2601;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;
const GL_VERTEX_SHADER: u32 = 0x8B31;
const GL_FRAGMENT_SHADER: u32 = 0x8B30;
const GL_COMPILE_STATUS: u32 = 0x8B81;
const GL_LINK_STATUS: u32 = 0x8B82;
const GL_FLOAT: u32 = 0x1406;
const GL_TRIANGLES: u32 = 0x0004;

/// The one vertex attribute: a corner of the triangle that covers the target.
const CORNER: u32 = 0;

const VERTEX_SHADER: &CStr = c"attribute vec2 corner;
varying vec2 coordinate;
void main() {
    coordinate = corner * 0.5 + 0.5;
    gl_Position = vec4(corner, 0.0, 1.0);
}";

const FRAGMENT_SHADER: &CStr = c"#extension GL_OES_EGL_image_external : require
precision mediump float;
uniform samplerExternalOES picture;
varying vec2 coordinate;
void main() {
    gl_FragColor = vec4(texture2D(picture, coordinate).rgb, 1.0);
}";

/// A triangle that covers the target, two coordinates a corner: the target's rows in memory
/// follow its coordinates' rising `y`, as the picture's do, so nothing is flipped.
const COVERING_TRIANGLE: [f32; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

/// One conversion asked of the drawing thread, and where its answer goes.
struct Conversion {
    picture: AndroidHardwareBuffer,
    answer: Sender<Result<AndroidHardwareBuffer, String>>,
}

/// The process's drawing thread, as its conversions reach it.
struct Converter(Mutex<Sender<Conversion>>);

/// `picture`, a camera's picture in its own format, drawn into a new RGBA8 buffer of its size; why
/// not, where the system could not draw it.
pub(super) fn to_rgba(picture: AndroidHardwareBuffer) -> Result<AndroidHardwareBuffer, String> {
    let converter =
        converter().ok_or("the system has no OpenGL ES to convert a camera's picture")?;
    let (answer, answered) = mpsc::channel();
    converter
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .send(Conversion { picture, answer })
        .map_err(|_| "the thread that converts camera pictures stopped".to_owned())?;
    answered
        .recv()
        .map_err(|_| "the thread that converts camera pictures stopped".to_owned())?
}

/// The drawing thread, started at the first conversion; `None` where it could not start.
fn converter() -> Option<&'static Converter> {
    static CONVERTER: OnceLock<Option<Converter>> = OnceLock::new();
    CONVERTER
        .get_or_init(|| {
            let (conversions, asked) = mpsc::channel();
            let (started, start) = mpsc::channel();
            std::thread::Builder::new()
                .name("waft camera frames".to_owned())
                .spawn(move || draw_conversions(asked, started))
                .ok()?;
            match start.recv() {
                Ok(Ok(())) => Some(Converter(Mutex::new(conversions))),
                Ok(Err(error)) => {
                    crate::log::warn_to_log!("waft: no OpenGL ES for camera frames: {error}");
                    None
                }
                Err(_) => None,
            }
        })
        .as_ref()
}

/// The drawing thread: makes its context current, says whether it could, and draws each
/// conversion asked until nothing can ask any more.
fn draw_conversions(asked: Receiver<Conversion>, started: Sender<Result<(), String>>) {
    let drawing = match Drawing::new() {
        Ok(drawing) => {
            let _ = started.send(Ok(()));
            drawing
        }
        Err(error) => {
            let _ = started.send(Err(error));
            return;
        }
    };
    for conversion in asked {
        let _ = conversion
            .answer
            .send(drawing.convert(conversion.picture.buffer()));
    }
}

/// What the drawing thread draws with, current on it.
struct Drawing {
    display: EglDisplay,
    program: u32,
    framebuffer: u32,
    extensions: Extensions,
}

/// EGL's and OpenGL ES's calls of their extensions, which are looked up.
struct Extensions {
    native_client_buffer: GetNativeClientBuffer,
    create_image: CreateImage,
    destroy_image: DestroyImage,
    image_target_texture: ImageTargetTexture,
}

impl Drawing {
    /// A context on the default display made current on this thread, with the program that
    /// samples a picture and the framebuffer it draws through.
    fn new() -> Result<Drawing, String> {
        // SAFETY: EGL's calls with the arguments its specification asks for, on this thread,
        // which keeps the context current for its life.
        unsafe {
            let display = eglGetDisplay(EGL_DEFAULT_DISPLAY);
            if display.is_null() || eglInitialize(display, ptr::null_mut(), ptr::null_mut()) == 0 {
                return Err(egl_error("eglInitialize"));
            }
            eglBindAPI(EGL_OPENGL_ES_API);
            let config = choose_config(display)?;
            let context_attributes = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
            let context =
                eglCreateContext(display, config, EGL_NO_CONTEXT, context_attributes.as_ptr());
            if context.is_null() {
                return Err(egl_error("eglCreateContext"));
            }
            let surface_attributes = [EGL_WIDTH, 1, EGL_HEIGHT, 1, EGL_NONE];
            let surface = eglCreatePbufferSurface(display, config, surface_attributes.as_ptr());
            if surface.is_null() || eglMakeCurrent(display, surface, surface, context) == 0 {
                return Err(egl_error("eglMakeCurrent"));
            }
            let extensions = Extensions::look_up()?;
            let program = program()?;
            let mut framebuffer = 0;
            glGenFramebuffers(1, &mut framebuffer);
            Ok(Drawing {
                display,
                program,
                framebuffer,
                extensions,
            })
        }
    }

    /// `picture` drawn into a new RGBA8 buffer of its size, the GPU done with it.
    fn convert(&self, picture: &HardwareBufferRef) -> Result<AndroidHardwareBuffer, String> {
        let description = picture.describe();
        let target = HardwareBuffer::allocate(HardwareBufferDesc {
            width: description.width,
            height: description.height,
            layers: 1,
            format: HardwareBufferFormat::R8G8B8A8_UNORM,
            usage: HardwareBufferUsage::GPU_COLOR_OUTPUT
                | HardwareBufferUsage::GPU_SAMPLED_IMAGE
                | HardwareBufferUsage::COMPOSER_OVERLAY
                | HardwareBufferUsage::CPU_READ_RARELY,
            stride: 0,
        })
        .map_err(|error| format!("no RGBA buffer for a camera's picture: {error}"))?;
        let source = self.image_of(picture)?;
        let drawn = self.image_of(&target).and_then(|destination| {
            let drawn = self.draw(source, destination, [description.width, description.height]);
            self.destroy(destination);
            drawn
        });
        self.destroy(source);
        drawn?;
        Ok(AndroidHardwareBuffer::new(target))
    }

    /// `buffer` as an EGL image, which the caller destroys.
    fn image_of(&self, buffer: &HardwareBufferRef) -> Result<EglImage, String> {
        let attributes = [EGL_IMAGE_PRESERVED_KHR, EGL_TRUE, EGL_NONE];
        // SAFETY: a live buffer, which the image holds a reference on, and the extension's
        // attributes.
        let image = unsafe {
            let client = (self.extensions.native_client_buffer)(buffer.as_ptr().cast());
            (self.extensions.create_image)(
                self.display,
                EGL_NO_CONTEXT,
                EGL_NATIVE_BUFFER_ANDROID,
                client,
                attributes.as_ptr(),
            )
        };
        if image.is_null() {
            return Err(egl_error("eglCreateImageKHR"));
        }
        Ok(image)
    }

    fn destroy(&self, image: EglImage) {
        // SAFETY: an image made by `image_of`, destroyed once.
        unsafe { (self.extensions.destroy_image)(self.display, image) };
    }

    /// Samples `source` over all of `destination`, `size` pixels large, and waits for the GPU.
    fn draw(&self, source: EglImage, destination: EglImage, size: [u32; 2]) -> Result<(), String> {
        // SAFETY: OpenGL ES's calls on the thread its context is current on, with textures made
        // and deleted here, and a client-side vertex array live across the draw.
        unsafe {
            let mut textures = [0; 2];
            glGenTextures(2, textures.as_mut_ptr());
            let [picture, target] = textures;
            glBindTexture(GL_TEXTURE_2D, target);
            (self.extensions.image_target_texture)(GL_TEXTURE_2D, destination);
            glBindFramebuffer(GL_FRAMEBUFFER, self.framebuffer);
            glFramebufferTexture2D(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                target,
                0,
            );
            let complete = glCheckFramebufferStatus(GL_FRAMEBUFFER) == GL_FRAMEBUFFER_COMPLETE;
            if complete {
                glActiveTexture(GL_TEXTURE0);
                glBindTexture(GL_TEXTURE_EXTERNAL_OES, picture);
                (self.extensions.image_target_texture)(GL_TEXTURE_EXTERNAL_OES, source);
                for (name, value) in [
                    (GL_TEXTURE_MIN_FILTER, GL_LINEAR),
                    (GL_TEXTURE_MAG_FILTER, GL_LINEAR),
                    (GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE),
                    (GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE),
                ] {
                    glTexParameteri(GL_TEXTURE_EXTERNAL_OES, name, value);
                }
                glUseProgram(self.program);
                glUniform1i(glGetUniformLocation(self.program, c"picture".as_ptr()), 0);
                glViewport(0, 0, size[0] as i32, size[1] as i32);
                glEnableVertexAttribArray(CORNER);
                glVertexAttribPointer(CORNER, 2, GL_FLOAT, 0, 0, COVERING_TRIANGLE.as_ptr().cast());
                glDrawArrays(GL_TRIANGLES, 0, 3);
                glFinish();
            }
            glFramebufferTexture2D(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, 0, 0);
            glBindFramebuffer(GL_FRAMEBUFFER, 0);
            glDeleteTextures(2, textures.as_ptr());
            if complete {
                Ok(())
            } else {
                Err("the RGBA buffer is not a framebuffer OpenGL ES draws into".to_owned())
            }
        }
    }
}

impl Extensions {
    fn look_up() -> Result<Extensions, String> {
        // SAFETY: each name is the extension's call, whose signature its type is.
        unsafe {
            Ok(Extensions {
                native_client_buffer: std::mem::transmute::<*mut c_void, GetNativeClientBuffer>(
                    proc_address(c"eglGetNativeClientBufferANDROID")?,
                ),
                create_image: std::mem::transmute::<*mut c_void, CreateImage>(proc_address(
                    c"eglCreateImageKHR",
                )?),
                destroy_image: std::mem::transmute::<*mut c_void, DestroyImage>(proc_address(
                    c"eglDestroyImageKHR",
                )?),
                image_target_texture: std::mem::transmute::<*mut c_void, ImageTargetTexture>(
                    proc_address(c"glEGLImageTargetTexture2DOES")?,
                ),
            })
        }
    }
}

/// The address of the extension call `name`; why not, where the system has none.
fn proc_address(name: &CStr) -> Result<*mut c_void, String> {
    // SAFETY: a nul-terminated name.
    let address = unsafe { eglGetProcAddress(name.as_ptr()) };
    if address.is_null() {
        return Err(format!("the system has no {}", name.to_string_lossy()));
    }
    Ok(address)
}

/// A config of 8-bit RGBA for OpenGL ES 2 with a pixel buffer surface.
///
/// # Safety
///
/// `display` is initialised.
unsafe fn choose_config(display: EglDisplay) -> Result<EglConfig, String> {
    let attributes = [
        EGL_RENDERABLE_TYPE,
        EGL_OPENGL_ES2_BIT,
        EGL_SURFACE_TYPE,
        EGL_PBUFFER_BIT,
        EGL_RED_SIZE,
        8,
        EGL_GREEN_SIZE,
        8,
        EGL_BLUE_SIZE,
        8,
        EGL_ALPHA_SIZE,
        8,
        EGL_NONE,
    ];
    let mut config = ptr::null_mut();
    let mut count = 0;
    // SAFETY: an initialised display, attributes ending in `EGL_NONE`, room for one config.
    let chosen =
        unsafe { eglChooseConfig(display, attributes.as_ptr(), &mut config, 1, &mut count) };
    if chosen == 0 || count < 1 {
        return Err(egl_error("eglChooseConfig"));
    }
    Ok(config)
}

/// The program that samples a picture through an external texture over a covering triangle.
///
/// # Safety
///
/// A context is current on this thread.
unsafe fn program() -> Result<u32, String> {
    // SAFETY: as the function says, and sources that are nul-terminated.
    unsafe {
        let vertex = shader(GL_VERTEX_SHADER, VERTEX_SHADER)?;
        let fragment = shader(GL_FRAGMENT_SHADER, FRAGMENT_SHADER)?;
        let program = glCreateProgram();
        glAttachShader(program, vertex);
        glAttachShader(program, fragment);
        glBindAttribLocation(program, CORNER, c"corner".as_ptr());
        glLinkProgram(program);
        let mut linked = 0;
        glGetProgramiv(program, GL_LINK_STATUS, &mut linked);
        if linked == 0 {
            return Err("the camera frames' program did not link".to_owned());
        }
        Ok(program)
    }
}

/// A shader of `kind` compiled from `source`.
///
/// # Safety
///
/// A context is current on this thread.
unsafe fn shader(kind: u32, source: &CStr) -> Result<u32, String> {
    // SAFETY: as the function says; the source is nul-terminated, so no length is given.
    unsafe {
        let shader = glCreateShader(kind);
        let sources = [source.as_ptr()];
        glShaderSource(shader, 1, sources.as_ptr(), ptr::null());
        glCompileShader(shader);
        let mut compiled = 0;
        glGetShaderiv(shader, GL_COMPILE_STATUS, &mut compiled);
        if compiled == 0 {
            return Err("a camera frames' shader did not compile".to_owned());
        }
        Ok(shader)
    }
}

fn egl_error(call: &str) -> String {
    // SAFETY: no arguments.
    format!("{call} failed: EGL error {:#x}", unsafe { eglGetError() })
}
