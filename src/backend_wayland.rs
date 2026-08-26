use crate::config::{Config, CrosshairStyle};
use crate::ipc::{self, Command, Response};
use crate::renderer::Renderer;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use wayland_client::{
    delegate_noop,
    globals::registry_queue_init,
    protocol::{wl_output, wl_region, wl_shm, wl_surface},
    Connection, QueueHandle,
};

pub struct WaylandApp {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,
    pub layer_surface: Option<LayerSurface>,
    pub pool: SlotPool,
    pub config: Config,
    pub width: u32,
    pub height: u32,
    pub buffer: Option<Buffer>,
    pub configured: bool,
    pub exit: bool,
}

impl WaylandApp {
    pub fn new(config: Config) -> Result<(Self, Connection, wayland_client::EventQueue<Self>), Box<dyn std::error::Error>> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh = event_queue.handle();

        let compositor_state = CompositorState::bind(&globals, &qh)?;
        let layer_shell = LayerShell::bind(&globals, &qh)?;
        let shm_state = Shm::bind(&globals, &qh)?;
        let output_state = OutputState::new(&globals, &qh);
        let registry_state = RegistryState::new(&globals);

        let width = 512;
        let height = 512;
        let pool = SlotPool::new((width * height * 4 * 2) as usize, &shm_state)?;

        let mut app = Self {
            registry_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            layer_surface: None,
            pool,
            config,
            width,
            height,
            buffer: None,
            configured: false,
            exit: false,
        };

        // Create surface
        let surface = app.compositor_state.create_surface(&qh);

        // Click-through: set an empty input region so all mouse clicks pass through
        let region = app.compositor_state.wl_compositor().create_region(&qh, ());
        surface.set_input_region(Some(&region));
        region.destroy();

        let layer_surface = app.layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Overlay,
            Some("crossover-overlay"),
            None,
        );

        // Center on screen with zero anchors
        layer_surface.set_anchor(Anchor::empty());
        layer_surface.set_size(app.width, app.height);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.commit();

        app.layer_surface = Some(layer_surface);

        Ok((app, conn, event_queue))
    }

    pub fn draw(&mut self) {
        let pixmap = match Renderer::render(&self.config, self.width) {
            Some(p) => p,
            None => return,
        };

        let width = self.width;
        let height = self.height;
        let stride = (width * 4) as i32;

        let (buffer, canvas) = match self.pool.create_buffer(
            width as i32,
            height as i32,
            stride,
            wl_shm::Format::Argb8888,
        ) {
            Ok(res) => res,
            Err(_) => return,
        };

        // Copy RGBA from tiny-skia to ARGB/BGRA for Wayland wl_shm
        let src_pixels = pixmap.data();
        for (src_chunk, dst_chunk) in src_pixels.chunks_exact(4).zip(canvas.chunks_exact_mut(4)) {
            let r = src_chunk[0];
            let g = src_chunk[1];
            let b = src_chunk[2];
            let a = src_chunk[3];
            dst_chunk[0] = b;
            dst_chunk[1] = g;
            dst_chunk[2] = r;
            dst_chunk[3] = a;
        }

        if let Some(layer_surface) = &self.layer_surface {
            let surface = layer_surface.wl_surface();
            let _ = buffer.attach_to(surface);
            surface.damage_buffer(0, 0, width as i32, height as i32);
            surface.commit();
        }

        self.buffer = Some(buffer);
    }
}

// Delegate handlers
delegate_compositor!(WaylandApp);
delegate_output!(WaylandApp);
delegate_shm!(WaylandApp);
delegate_layer!(WaylandApp);
delegate_registry!(WaylandApp);
delegate_noop!(WaylandApp: ignore wl_region::WlRegion);

impl CompositorHandler for WaylandApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_scale: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ShmHandler for WaylandApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl LayerShellHandler for WaylandApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.configured = true;
        self.draw();
    }
}

impl ProvidesRegistryState for WaylandApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

pub struct IpcTask {
    pub cmd: Command,
    pub resp_tx: Sender<Response>,
}

pub fn run_wayland(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, conn, mut event_queue) = WaylandApp::new(config)?;

    let (task_tx, task_rx): (Sender<IpcTask>, Receiver<IpcTask>) = channel();
    let listener = ipc::create_listener()?;

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    if let Ok(cmd) = serde_json::from_str::<Command>(line.trim()) {
                        let (resp_tx, resp_rx) = channel();
                        let _ = task_tx.send(IpcTask { cmd, resp_tx });
                        if let Ok(resp) = resp_rx.recv() {
                            if let Ok(json) = serde_json::to_string(&resp) {
                                let _ = stream.write_all(json.as_bytes());
                                let _ = stream.write_all(b"\n");
                            }
                        }
                    }
                }
            }
        }
    });

    // Do initial roundtrip to process surface configure and draw initial crosshair
    let _ = event_queue.roundtrip(&mut app);
    app.draw();
    let _ = conn.flush();

    // Steady state event loop
    while !app.exit {
        let _ = event_queue.dispatch_pending(&mut app);

        let mut needs_redraw = false;

        // Handle IPC commands
        while let Ok(task) = task_rx.try_recv() {
            match task.cmd {
                Command::Toggle => {
                    app.config.visible = !app.config.visible;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::Show => {
                    app.config.visible = true;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::Hide => {
                    app.config.visible = false;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::SetStyle(s) => {
                    app.config.style = CrosshairStyle::from_str(&s);
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::SetSize(sz) => {
                    app.config.size = sz;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::SetColor(hex) => {
                    if let Some(rgba) = parse_hex_color(&hex) {
                        app.config.color = rgba;
                        let _ = app.config.save();
                        needs_redraw = true;
                    }
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::SetThickness(th) => {
                    app.config.thickness = th;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::SetGap(g) => {
                    app.config.gap = g;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::Nudge { dx, dy } => {
                    app.config.offset_x += dx;
                    app.config.offset_y += dy;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::ResetPosition => {
                    app.config.offset_x = 0;
                    app.config.offset_y = 0;
                    let _ = app.config.save();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::GetStatus => {
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::Reload => {
                    app.config = Config::load();
                    needs_redraw = true;
                    let _ = task.resp_tx.send(Response::Status(app.config.clone()));
                }
                Command::Quit => {
                    let _ = task.resp_tx.send(Response::Ok);
                    app.exit = true;
                }
                Command::Ping => {
                    let _ = task.resp_tx.send(Response::Pong);
                }
            }
        }

        if needs_redraw {
            app.draw();
            let _ = conn.flush();
        }

        thread::sleep(Duration::from_millis(15));
    }

    let _ = std::fs::remove_file(ipc::socket_path());
    Ok(())
}

fn parse_hex_color(hex: &str) -> Option<[u8; 4]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some([r, g, b, 255])
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        Some([r, g, b, a])
    } else {
        None
    }
}
