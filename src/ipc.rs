use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Toggle,
    Show,
    Hide,
    SetStyle(String),
    SetSize(u32),
    SetColor(String),
    SetThickness(u32),
    SetGap(u32),
    Nudge { dx: i32, dy: i32 },
    ResetPosition,
    GetStatus,
    Reload,
    Quit,
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Status(Config),
    Pong,
    Error(String),
}

pub fn socket_path() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/tmp/crossover-{}.sock", uid))
}

pub fn is_daemon_running() -> bool {
    let path = socket_path();
    if !path.exists() {
        return false;
    }

    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));

        if let Ok(msg) = serde_json::to_string(&Command::Ping) {
            if stream.write_all(msg.as_bytes()).is_ok() && stream.write_all(b"\n").is_ok() {
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() {
                    if let Ok(Response::Pong) = serde_json::from_str::<Response>(line.trim()) {
                        return true;
                    }
                }
            }
        }
    }

    // If socket exists but does not respond, it's stale -> remove it
    let _ = std::fs::remove_file(&path);
    false
}

pub fn send_command(cmd: &Command) -> Result<Response, Box<dyn std::error::Error>> {
    let path = socket_path();
    let mut stream = UnixStream::connect(path)?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let msg = serde_json::to_string(cmd)?;
    stream.write_all(msg.as_bytes())?;
    stream.write_all(b"\n")?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let resp: Response = serde_json::from_str(line.trim())?;
    Ok(resp)
}

pub fn create_listener() -> Result<UnixListener, std::io::Error> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(path)
}
