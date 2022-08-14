use std::{error::Error, result, time};

use log::{debug, error};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::{
    camera::{CameraController, CameraOrthographic},
    frame_counter::{ExponentialMovingAverage, FPSPrinter, FrameCounter},
    input::InputSystem,
    object::GameObject,
    renderer::{backend::renderer::VulkanRenderer, frontend, Renderer2DSystem},
};

type Result<T> = result::Result<T, Box<dyn Error>>;

#[derive(Default)]
pub struct EngineBuilder {
    app: Option<Box<dyn Application>>,
    wb: Option<WindowBuilder>,
}

impl EngineBuilder {
    /// Initializes a new `EngineBuilder` with default values.
    #[inline]
    pub fn new(app: Box<dyn Application>) -> Self {
        let wb = WindowBuilder::new();
        Self {
            app: Some(app),
            wb: Some(wb),
        }
    }

    #[inline]
    pub fn with_application(mut self, app: Option<Box<dyn Application>>) -> Self {
        self.app = app;
        self
    }

    #[inline]
    pub fn with_window_builder(mut self, wb: Option<WindowBuilder>) -> Self {
        self.wb = wb;
        self
    }

    #[inline]
    pub fn build(mut self) -> Result<Engine> {
        let app = self.app.take().ok_or("app is None")?;
        let wb = self.wb.take().ok_or("window builder is None")?;

        Ok(Engine::new(app, wb))
    }
}

pub struct Engine {
    application: Option<Box<dyn Application>>,
    window_builder: Option<WindowBuilder>,
}

impl Engine {
    /// Initializes a new `Engine` with provided values.
    #[inline]
    pub fn new(app: Box<dyn Application>, wb: WindowBuilder) -> Self {
        Self {
            application: Some(app),
            window_builder: Some(wb),
        }
    }

    pub fn run(&mut self) {
        // take ownership of struct attributes
        let mut application = self
            .application
            .take()
            .ok_or("app is None")
            .expect("take app");
        let window_builder = self
            .window_builder
            .take()
            .ok_or("window builder is None")
            .expect("take window builder");

        // window
        let event_loop = EventLoop::new();
        let window = window_builder
            .build(&event_loop)
            .expect("window builder builds");

        // camera system
        let mut camera_controller = {
            let PhysicalSize { width, height } = window.inner_size();
            let camera = CameraOrthographic::new(width, height);
            CameraController::new(camera)
        };

        // input system
        let mut input = InputSystem::new();

        // renderer system
        let mut vulkan_renderer =
            unsafe { VulkanRenderer::new("Engine", &window).expect("create vulkan renderer") };

        let mut renderer2d_system = unsafe {
            Renderer2DSystem::new(vulkan_renderer.device(), vulkan_renderer.renderpass())
                .expect("create renderer2D system")
        };

        // ImGui
        let (mut winit_platform, mut imgui_context) = frontend::imgui::init(&window);
        let mut imgui_renderer = unsafe {
            frontend::imgui::Renderer::new(
                &mut imgui_context,
                vulkan_renderer.device(),
                vulkan_renderer.renderpass(),
            )
            .expect("initialize imgui renderer")
        };

        // frame counter system
        let mut frame_counter = FrameCounter::new();

        // fps printer system
        let mut fps_printer = {
            let moving_average = ExponentialMovingAverage::new().with_alpha(0.95);
            let print_fn = |fps| debug!("fps: {:.2}", fps);
            FPSPrinter::new(moving_average, print_fn).with_throttle_ms(500)
        };

        // game objects
        let mut objects = Vec::new();

        // run application initialization
        application.on_init(ApplicationContext::new(
            &mut objects,
            frame_counter.delta_time(),
        ));

        // run main loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            // update ImGui system
            winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
            // update input system
            input.on_event(&event);
            // update camera system
            camera_controller.on_event(&event);

            match event {
                // handle close window
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,

                // Emitted when new events arrive from the OS to be processed.
                // This event type is useful as a place to put code that should be done before you
                // start processing events.
                Event::NewEvents(_) => {
                    frame_counter.on_update(time::Instant::now());
                    // update ImGui delta time
                    imgui_context
                        .io_mut()
                        .update_delta_time(frame_counter.delta_time());
                }

                // handle window resize
                Event::WindowEvent {
                    event: WindowEvent::Resized(PhysicalSize { width, height }),
                    ..
                } => vulkan_renderer.resize(width, height),

                // handle shutdown
                Event::LoopDestroyed => unsafe {
                    renderer2d_system.destroy(vulkan_renderer.device());
                    imgui_renderer.destroy(vulkan_renderer.device(), &mut imgui_context);
                    vulkan_renderer.destroy();
                },

                // NOTE: the MainEventsCleared event will be emitted when all input events
                //       have been processed and redraw processing is about to begin.
                Event::MainEventsCleared => {
                    let delta_time = frame_counter.delta_time();

                    // print fps
                    fps_printer.on_update(delta_time, frame_counter.fps());

                    // update application state
                    application.on_update(ApplicationContext::new(&mut objects, delta_time));

                    // update camera
                    camera_controller.on_update(&input, delta_time);

                    // render
                    unsafe {
                        if vulkan_renderer.begin_frame().expect("begin frame succeeds") {
                            if let Err(e) = vulkan_renderer.draw(|_, command_buffer| {
                                // Renderer 2D
                                renderer2d_system
                                    .render(
                                        vulkan_renderer.device(),
                                        command_buffer,
                                        delta_time,
                                        camera_controller.view_projection_matrix(),
                                        &objects,
                                    )
                                    .expect("renderer 2D render");

                                // ImGui
                                winit_platform
                                    .prepare_frame(imgui_context.io_mut(), &window)
                                    .expect("prepare ImGui frame");
                                let ui = imgui_context.new_frame();
                                ui.show_demo_window(&mut true);
                                winit_platform.prepare_render(ui, &window);
                                imgui_renderer
                                    .render(
                                        vulkan_renderer.device(),
                                        command_buffer,
                                        imgui_context.render(),
                                    )
                                    .expect("imgui renderer render");
                            }) {
                                error!("draw {e:?}");
                            }

                            vulkan_renderer.end_frame().expect("end frame succeeds");
                        }
                    }
                }

                // catch-all
                _ => (),
            }
        });
    }
}

pub struct ApplicationContext<'a> {
    objects: &'a mut Vec<GameObject>,
    delta_time: time::Duration,
}

impl<'a> ApplicationContext<'a> {
    fn new(objects: &'a mut Vec<GameObject>, delta_time: time::Duration) -> Self {
        Self {
            objects,
            delta_time,
        }
    }

    pub fn delta_time(&self) -> time::Duration {
        self.delta_time
    }

    pub fn add_object(&mut self, object: GameObject) {
        self.objects.push(object);
    }
}

pub trait Application {
    fn on_init(&mut self, ctx: ApplicationContext);
    fn on_update(&mut self, ctx: ApplicationContext);
}
