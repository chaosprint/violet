use std::time::Instant;

use flax::{components::name, Schedule, World};
use glam::{vec2, Vec2};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

use crate::{
    assets::AssetCache,
    components::{self, local_position, rect, screen_position, Rect},
    executor::Executor,
    input::InputState,
    systems::{layout_system, templating_system, transform_system},
    wgpu::{
        graphics::Gpu,
        systems::{load_fonts_system, update_text_heuristics},
        window_renderer::WindowRenderer,
    },
    Frame, Widget,
};

pub struct Canvas<W> {
    size: Vec2,
    root: W,
}

impl<W: Widget> Widget for Canvas<W> {
    fn mount(self, scope: &mut crate::Scope<'_>) {
        scope
            .set(name(), "Canvas".into())
            .set(
                rect(),
                Rect {
                    min: Vec2::ZERO,
                    max: self.size,
                },
            )
            .set_default(screen_position())
            .set_default(local_position());

        scope.attach(self.root);
    }
}

pub struct App {}

impl App {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(self, root: impl Widget) -> anyhow::Result<()> {
        let mut ex = Executor::new();

        let spawner = ex.spawner();

        let world = World::new();

        let mut frame = Frame {
            world,
            spawner,
            assets: AssetCache::new(),
            delta_time: 0.0,
        };

        let event_loop = EventLoopBuilder::new().build();

        let window = WindowBuilder::new().build(&event_loop)?;
        let window_size = window.inner_size();
        let window_size = vec2(window_size.width as f32, window_size.height as f32);

        let mut input_state = InputState::new(Vec2::ZERO);

        // Mount the root widget
        let root = frame.new_root(Canvas {
            size: window_size,
            root,
        });

        // TODO: Make this a proper effect
        let (gpu, surface) = futures::executor::block_on(Gpu::with_surface(window));

        let mut window_renderer = WindowRenderer::new(gpu, &mut frame, surface);

        let mut schedule = Schedule::new()
            .with_system(templating_system(root))
            .flush()
            .with_system(load_fonts_system(frame.assets.clone()))
            .flush()
            .with_system(update_text_heuristics())
            .with_system(layout_system())
            .with_system(transform_system());

        let mut cur_time = Instant::now();

        event_loop.run(move |event, _, ctl| match event {
            Event::MainEventsCleared => {
                let new_time = Instant::now();

                let dt = new_time.duration_since(cur_time);
                let delta_time = dt.as_secs_f32();

                cur_time = new_time;

                frame.delta_time = delta_time;

                // tracing::info!(?dt, fps = 1.0 / delta_time);

                ex.tick(&mut frame);

                schedule.execute_seq(&mut frame.world).unwrap();

                if let Err(err) = window_renderer.draw(&mut frame) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }
            }
            Event::RedrawRequested(_) => {
                tracing::info!("Redraw requested");
                if let Err(err) = window_renderer.draw(&mut frame) {
                    tracing::error!("Failed to draw to window: {err:?}");
                    *ctl = ControlFlow::Exit
                }
            }
            Event::WindowEvent { window_id, event } => match event {
                WindowEvent::MouseInput { state, button, .. } => {
                    input_state.on_mouse_input(&mut frame, state, button);
                }
                WindowEvent::KeyboardInput {
                    input,
                    is_synthetic,
                    ..
                } => input_state.on_keyboard_input(&mut frame, input),
                WindowEvent::CursorMoved { position, .. } => {
                    input_state.on_cursor_move(vec2(position.x as f32, position.y as f32))
                }
                WindowEvent::Resized(size) => {
                    frame
                        .world_mut()
                        .set(
                            root,
                            components::rect(),
                            Rect {
                                min: vec2(0.0, 0.0),
                                max: vec2(size.width as f32, size.height as f32),
                            },
                        )
                        .unwrap();

                    window_renderer.resize(size);
                }
                WindowEvent::CloseRequested => {
                    *ctl = ControlFlow::Exit;
                }
                event => {
                    tracing::trace!(?event, ?window_id, "Window event")
                }
            },
            event => {
                tracing::trace!(?event, "Event")
            }
        })
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
