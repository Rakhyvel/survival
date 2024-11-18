use std::cell::RefCell;
use std::time::Instant;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Scancode;
use sdl2::sys::{SDL_GetPerformanceCounter, SDL_GetPerformanceFrequency};
use sdl2::video::SwapInterval;
use sdl2::Sdl;

use super::render_core::RenderContext;

pub struct App {
    // Screen stuff
    pub window_size: nalgebra_glm::I32Vec2,
    pub renderer: RenderContext,

    // Main loop stuff
    pub running: bool,
    pub seconds: f32, //< How many seconds the program has been up
    pub ticks: usize, //< How many ticks the program has been up

    // User input state
    pub keys: [bool; 256],
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_rel_x: i32,
    pub mouse_rel_y: i32,
    pub mouse_left_down: bool,
    pub mouse_right_down: bool,
    prev_mouse_left_down: bool,
    prev_mouse_right_down: bool,
    pub mouse_left_clicked: bool,
    pub mouse_right_clicked: bool,
    pub mouse_wheel: f32,
}

pub fn run(
    window_size: nalgebra_glm::I32Vec2,
    window_title: &'static str,
    init: &dyn Fn(&App) -> RefCell<Box<dyn Scene>>,
) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _audio_subsystem = sdl_context.audio()?;

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    gl_attr.set_double_buffer(true);

    let window = video_subsystem
        .window(window_title, window_size.x as u32, window_size.y as u32)
        .resizable()
        .opengl()
        .build()
        .unwrap();

    let _gl_context = window.gl_create_context().unwrap();

    let _gl =
        gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);

    window
        .subsystem()
        .gl_set_swap_interval(SwapInterval::VSync)
        .unwrap();

    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);
        gl::Enable(gl::CULL_FACE);
        gl::Enable(gl::MULTISAMPLE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        // gl::Enable(gl::FRAMEBUFFER_SRGB);
    }

    let mut app = App {
        window_size,
        renderer: RenderContext::default(),
        // sdl_context,
        running: true,
        keys: [false; 256],
        mouse_x: 0,
        mouse_y: 0,
        mouse_rel_x: 0,
        mouse_rel_y: 0,
        mouse_left_down: false,
        mouse_right_down: false,
        prev_mouse_left_down: false,
        prev_mouse_right_down: false,
        mouse_left_clicked: false,
        mouse_right_clicked: false,
        mouse_wheel: 0.0,
        seconds: 0.0,
        ticks: 0,
    };

    let initial_scene = init(&app);
    let mut scene_stack: Vec<RefCell<Box<dyn Scene>>> = vec![];
    scene_stack.push(initial_scene);

    let time = Instant::now();
    let mut start = time.elapsed().as_millis();
    let mut current;
    let mut previous = 0;
    let mut lag = 0;
    let mut elapsed;
    let mut frames = 0;
    const DELTA_T: u128 = 16;
    while app.running {
        app.seconds = time.elapsed().as_secs_f32();
        current = time.elapsed().as_millis();
        elapsed = current - previous;

        previous = current;
        lag += elapsed;

        let scene_stale = false;
        while lag >= DELTA_T {
            app.reset_input();
            app.poll_input(&sdl_context);
            // sdl_context.mouse().warp_mouse_in_window(
            //     &window,
            //     app.screen_width / 2,
            //     app.screen_height / 2,
            // );
            // sdl_context.mouse().set_relative_mouse_mode(true);

            if let Some(scene_ref) = scene_stack.last() {
                scene_ref.borrow_mut().update(&app);
                app.ticks += 1;
            }

            if !scene_stale {
                // if scene isn't stale, purge the scene
                lag -= DELTA_T;
            } else {
                break;
            }
        }

        if !scene_stale {
            app.renderer.int_screen_resolution = app.window_size;
            if let Some(scene_ref) = scene_stack.last() {
                scene_ref.borrow_mut().render(&app);
                frames += 1;
            }
            window.gl_swap_window();
        }

        let end = unsafe { SDL_GetPerformanceCounter() };
        let freq = unsafe { SDL_GetPerformanceFrequency() };
        let seconds = (end as f64 - (start as f64)) / (freq as f64);
        if seconds > 5.0 {
            println!("5 seconds;  fps: {}", frames / 5);
            start = end as u128;
            frames = 0;
        }
    }

    Ok(())
}

impl App {
    fn reset_input(&mut self) {
        self.mouse_rel_x = 0;
        self.mouse_rel_y = 0;
        self.mouse_wheel = 0.0;
        self.prev_mouse_left_down = self.mouse_left_down;
        self.prev_mouse_right_down = self.mouse_right_down;
    }

    fn poll_input(&mut self, sdl_context: &Sdl) {
        let mut event_queue = sdl_context.event_pump().unwrap();
        for event in event_queue.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    self.running = false;
                }

                Event::MouseMotion {
                    x, y, xrel, yrel, ..
                } => {
                    self.mouse_x = x;
                    self.mouse_y = y;
                    self.mouse_rel_x = xrel;
                    self.mouse_rel_y = yrel;
                }

                Event::MouseButtonDown { mouse_btn, .. } => match mouse_btn {
                    sdl2::mouse::MouseButton::Left => self.mouse_left_down = true,
                    sdl2::mouse::MouseButton::Right => self.mouse_right_down = true,
                    _ => {}
                },

                Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    sdl2::mouse::MouseButton::Left => self.mouse_left_down = false,
                    sdl2::mouse::MouseButton::Right => self.mouse_right_down = false,
                    _ => {}
                },

                Event::MouseWheel { y, .. } => {
                    self.mouse_wheel = y as f32;
                }

                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(new_width, new_height) = win_event {
                        self.window_size = nalgebra_glm::I32Vec2::new(new_width, new_height)
                    }
                }

                Event::KeyDown { scancode, .. } => match scancode {
                    Some(sc) => {
                        self.keys[sc as usize] = true;
                        if self.keys[Scancode::Escape as usize] {
                            self.running = false
                        }
                    }
                    None => {}
                },

                Event::KeyUp { scancode, .. } => match scancode {
                    Some(sc) => self.keys[sc as usize] = false,
                    None => {}
                },

                _ => {}
            }
        }

        self.mouse_left_clicked = !self.prev_mouse_left_down && self.mouse_left_down;
        self.mouse_right_clicked = !self.prev_mouse_right_down && self.mouse_right_down;
    }
}

pub trait Scene {
    // TODO: Return a "command" enum so that scene's can affect App state
    fn update(&mut self, app: &App);
    fn render(&mut self, app: &App);
}
