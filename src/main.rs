pub mod config;
pub mod interactive;
pub mod ipc;
pub mod renderer;
pub mod svg_assets;

#[cfg(feature = "wayland")]
pub mod backend_wayland;

#[cfg(feature = "x11")]
pub mod backend_x11;

use config::Config;
use ipc::{is_daemon_running, send_command, Command, Response};
use std::env;
use std::thread;
use std::time::Duration;

fn print_help() {
    println!(
        r##"crossover - Ultra-lightweight Crosshair Overlay for Linux (207+ Reticles Embedded)

Modes:
  crossover                 Launch interactive terminal controller (TUI)
  crossover -i, --interactive Force interactive terminal mode
  crossover -d, --daemon    Start overlay daemon directly (for autostart / scripts)

CLI Commands (Background Control):
  crossover --toggle        Toggle crosshair visibility
  crossover --show          Show crosshair
  crossover --hide          Hide crosshair
  crossover --style <name>  Set style (cross, dot, circle, circledot, chevron, box, tstyle, or 0-0..9-19)
  crossover --size <px>     Set crosshair size (e.g. 32)
  crossover --thickness <px> Set line thickness (e.g. 2)
  crossover --gap <px>      Set center gap (e.g. 5)
  crossover --color <hex>   Set color (e.g. #00ff88 or #ff0000aa)
  crossover --nudge-up      Move crosshair 1px up
  crossover --nudge-down    Move crosshair 1px down
  crossover --nudge-left    Move crosshair 1px left
  crossover --nudge-right   Move crosshair 1px right
  crossover --reset         Reset position to center
  crossover --status        Show current overlay status
  crossover --list          List all 207 embedded crosshair styles
  crossover --reload        Reload ~/.config/crossover/config.toml
  crossover --quit          Close overlay daemon
"##
    );
}

fn ensure_daemon_running() {
    if !is_daemon_running() {
        if let Ok(exe) = env::current_exe() {
            let _ = std::process::Command::new(exe)
                .arg("-d")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
        for _ in 0..25 {
            thread::sleep(Duration::from_millis(40));
            if is_daemon_running() {
                break;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let first = args[1].as_str();
        match first {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--list" | "-l" => {
                let styles = interactive::get_all_style_keys();
                println!("🎯 Available Crosshairs (Total: {}):", styles.len());
                for (i, k) in styles.iter().enumerate() {
                    println!("  {:3}. {:<25} (use: crossover --style {})", i + 1, k, k);
                }
                return Ok(());
            }
            "-d" | "--daemon" => {
                if is_daemon_running() {
                    println!("[crossover] Daemon is already running.");
                    return Ok(());
                }
                let config = Config::load();
                let is_wayland = env::var("WAYLAND_DISPLAY").is_ok()
                    || env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false);

                #[cfg(feature = "wayland")]
                if is_wayland {
                    if let Err(e) = backend_wayland::run_wayland(config.clone()) {
                        eprintln!("[crossover] Wayland native Layer-Shell not supported by compositor: {}. Falling back to X11/XWayland...", e);
                    } else {
                        return Ok(());
                    }
                }

                #[cfg(feature = "x11")]
                {
                    return backend_x11::run_x11(config);
                }

                #[allow(unreachable_code)]
                {
                    eprintln!("[crossover] Error: No display backend available.");
                    return Ok(());
                }
            }
            "-i" | "--interactive" => {
                ensure_daemon_running();
                return interactive::run_interactive();
            }
            "--toggle" | "-t" => {
                match send_command(&Command::Toggle) {
                    Ok(Response::Status(cfg)) => {
                        println!("Crosshair visibility toggled (visible: {})", cfg.visible)
                    }
                    Ok(_) => println!("Crosshair visibility toggled"),
                    Err(e) => eprintln!("Error (is daemon running?): {}", e),
                }
                return Ok(());
            }
            "--show" => {
                let _ = send_command(&Command::Show);
                println!("Crosshair is now visible");
                return Ok(());
            }
            "--hide" => {
                let _ = send_command(&Command::Hide);
                println!("Crosshair is now hidden");
                return Ok(());
            }
            "--style" => {
                if let Some(style) = args.get(2) {
                    let _ = send_command(&Command::SetStyle(style.clone()));
                    println!("Style set to: {}", style);
                } else {
                    eprintln!("Missing style name. Run `crossover --list` to see all styles.");
                }
                return Ok(());
            }
            "--size" => {
                if let Some(s) = args.get(2).and_then(|v| v.parse::<u32>().ok()) {
                    let _ = send_command(&Command::SetSize(s));
                    println!("Size set to: {}px", s);
                } else {
                    eprintln!("Missing or invalid size value");
                }
                return Ok(());
            }
            "--thickness" => {
                if let Some(th) = args.get(2).and_then(|v| v.parse::<u32>().ok()) {
                    let _ = send_command(&Command::SetThickness(th));
                    println!("Thickness set to: {}px", th);
                }
                return Ok(());
            }
            "--gap" => {
                if let Some(g) = args.get(2).and_then(|v| v.parse::<u32>().ok()) {
                    let _ = send_command(&Command::SetGap(g));
                    println!("Gap set to: {}px", g);
                }
                return Ok(());
            }
            "--color" => {
                if let Some(c) = args.get(2) {
                    let _ = send_command(&Command::SetColor(c.clone()));
                    println!("Color set to: {}", c);
                }
                return Ok(());
            }
            "--nudge-up" => {
                let _ = send_command(&Command::Nudge { dx: 0, dy: -1 });
                return Ok(());
            }
            "--nudge-down" => {
                let _ = send_command(&Command::Nudge { dx: 0, dy: 1 });
                return Ok(());
            }
            "--nudge-left" => {
                let _ = send_command(&Command::Nudge { dx: -1, dy: 0 });
                return Ok(());
            }
            "--nudge-right" => {
                let _ = send_command(&Command::Nudge { dx: 1, dy: 0 });
                return Ok(());
            }
            "--reset" => {
                let _ = send_command(&Command::ResetPosition);
                println!("Position reset to center");
                return Ok(());
            }
            "--status" => {
                match send_command(&Command::GetStatus) {
                    Ok(Response::Status(cfg)) => {
                        println!("Status: {}", if cfg.visible { "Visible" } else { "Hidden" });
                        println!("Style: {}", cfg.style.to_string_repr());
                        println!("Size: {}px, Thickness: {}px, Gap: {}px", cfg.size, cfg.thickness, cfg.gap);
                        println!("Color: #{:02X}{:02X}{:02X}", cfg.color[0], cfg.color[1], cfg.color[2]);
                        println!("Offset: [X: {}, Y: {}]", cfg.offset_x, cfg.offset_y);
                    }
                    _ => eprintln!("Daemon is not running. Start it with `crossover` or `crossover -d`"),
                }
                return Ok(());
            }
            "--reload" => {
                let _ = send_command(&Command::Reload);
                println!("Configuration reloaded from ~/.config/crossover/config.toml");
                return Ok(());
            }
            "--quit" | "-q" => {
                let _ = send_command(&Command::Quit);
                println!("Overlay stopped.");
                return Ok(());
            }
            _ => {
                print_help();
                return Ok(());
            }
        }
    }

    ensure_daemon_running();
    interactive::run_interactive()
}
