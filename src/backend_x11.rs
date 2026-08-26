use crate::config::{Config, CrosshairStyle};
use crate::ipc::{self, Command, Response};
use crate::renderer::Renderer;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::shape::SK;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::protocol::xproto::*;

pub struct IpcTask {
    pub cmd: Command,
    pub resp_tx: Sender<Response>,
}

pub fn run_x11(mut config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];

    // Find 32-bit visual for alpha transparency
    let mut visual_id = screen.root_visual;
    let mut depth = 32;
    let mut found_32 = false;

    for d in &screen.allowed_depths {
        if d.depth == 32 {
            if let Some(v) = d.visuals.first() {
                visual_id = v.visual_id;
                found_32 = true;
                break;
            }
        }
    }

    if !found_32 {
        depth = screen.root_depth;
        visual_id = screen.root_visual;
    }

    let colormap_id = conn.generate_id()?;
    conn.create_colormap(
        ColormapAlloc::NONE,
        colormap_id,
        screen.root,
        visual_id,
    )?;

    let win_id = conn.generate_id()?;
    let width: u16 = 512;
    let height: u16 = 512;
    let screen_w = screen.width_in_pixels;
    let screen_h = screen.height_in_pixels;
    let x = (screen_w as i16 - width as i16) / 2 + config.offset_x as i16;
    let y = (screen_h as i16 - height as i16) / 2 + config.offset_y as i16;

    let win_aux = CreateWindowAux::new()
        .background_pixel(0)
        .border_pixel(0)
        .colormap(colormap_id)
        .override_redirect(1)
        .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY);

    conn.create_window(
        depth,
        win_id,
        screen.root,
        x,
        y,
        width,
        height,
        0,
        WindowClass::INPUT_OUTPUT,
        visual_id,
        &win_aux,
    )?;

    // Click-through: set input shape to empty region
    let region_id = conn.generate_id()?;
    conn.xfixes_create_region(region_id, &[])?;
    conn.xfixes_set_window_shape_region(win_id, SK::INPUT, 0, 0, region_id)?;
    conn.xfixes_destroy_region(region_id)?;

    // Graphics context
    let gc_id = conn.generate_id()?;
    conn.create_gc(gc_id, win_id, &CreateGCAux::new())?;

    // Map (show) window
    conn.map_window(win_id)?;
    conn.flush()?;

    // Redraw closure
    let redraw = |conn: &x11rb::rust_connection::RustConnection, cfg: &Config| -> Result<(), Box<dyn std::error::Error>> {
        let pixmap = match Renderer::render(cfg, width as u32) {
            Some(p) => p,
            None => return Ok(()),
        };

        // Convert RGBA to BGRA for X11 32-bit TrueColor
        let mut bgra_data = Vec::with_capacity((width as usize) * (height as usize) * 4);
        for chunk in pixmap.data().chunks_exact(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];
            bgra_data.push(b);
            bgra_data.push(g);
            bgra_data.push(r);
            bgra_data.push(a);
        }

        conn.clear_area(false, win_id, 0, 0, width, height)?;
        conn.put_image(
            ImageFormat::Z_PIXMAP,
            win_id,
            gc_id,
            width,
            height,
            0,
            0,
            0,
            depth,
            &bgra_data,
        )?;
        conn.flush()?;
        Ok(())
    };

    redraw(&conn, &config)?;

    // Start IPC thread
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

    loop {
        // Poll IPC commands
        while let Ok(task) = task_rx.try_recv() {
            match task.cmd {
                Command::Toggle => {
                    config.visible = !config.visible;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::Show => {
                    config.visible = true;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::Hide => {
                    config.visible = false;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::SetStyle(s) => {
                    config.style = CrosshairStyle::from_str(&s);
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::SetSize(sz) => {
                    config.size = sz;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::SetColor(hex) => {
                    if let Some(rgba) = parse_hex_color(&hex) {
                        config.color = rgba;
                        let _ = config.save();
                        redraw(&conn, &config)?;
                    }
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::SetThickness(th) => {
                    config.thickness = th;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::SetGap(g) => {
                    config.gap = g;
                    let _ = config.save();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::Nudge { dx, dy } => {
                    config.offset_x += dx;
                    config.offset_y += dy;
                    let _ = config.save();
                    let nx = (screen_w as i16 - width as i16) / 2 + config.offset_x as i16;
                    let ny = (screen_h as i16 - height as i16) / 2 + config.offset_y as i16;
                    conn.configure_window(
                        win_id,
                        &ConfigureWindowAux::new().x(nx as i32).y(ny as i32),
                    )?;
                    conn.flush()?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::ResetPosition => {
                    config.offset_x = 0;
                    config.offset_y = 0;
                    let _ = config.save();
                    let nx = (screen_w as i16 - width as i16) / 2;
                    let ny = (screen_h as i16 - height as i16) / 2;
                    conn.configure_window(
                        win_id,
                        &ConfigureWindowAux::new().x(nx as i32).y(ny as i32),
                    )?;
                    conn.flush()?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::GetStatus => {
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::Reload => {
                    config = Config::load();
                    redraw(&conn, &config)?;
                    let _ = task.resp_tx.send(Response::Status(config.clone()));
                }
                Command::Quit => {
                    let _ = task.resp_tx.send(Response::Ok);
                    let _ = std::fs::remove_file(ipc::socket_path());
                    return Ok(());
                }
                Command::Ping => {
                    let _ = task.resp_tx.send(Response::Pong);
                }
            }
        }

        // Check X11 events
        while let Some(event) = conn.poll_for_event()? {
            if let x11rb::protocol::Event::Expose(_) = event {
                redraw(&conn, &config)?;
            }
        }

        thread::sleep(Duration::from_millis(10));
    }
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
