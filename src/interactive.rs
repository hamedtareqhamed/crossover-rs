use crate::config::Config;
use crate::ipc::{send_command, Command, Response};
use crate::svg_assets::KENNEY_SVGS;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use std::io::{stdout, Write};
use std::time::Duration;

#[derive(PartialEq)]
pub enum Mode {
    Nudge,   // Arrow keys move crosshair 1px
    Command, // VSCode-style command autocomplete menu
}

#[derive(Clone)]
pub struct SuggestionItem {
    pub text: String,
    pub category: &'static str,
}

pub fn get_all_suggestions() -> Vec<SuggestionItem> {
    let mut items = Vec::new();

    // 1. Built-in Core Styles
    let core_styles = [
        "style cross",
        "style dot",
        "style circle",
        "style circledot",
        "style chevron",
        "style box",
        "style tstyle",
    ];
    for s in core_styles {
        items.push(SuggestionItem {
            text: s.to_string(),
            category: "Core Style",
        });
    }

    // 2. Common Colors
    let colors = [
        "color #00ff88",
        "color #ff0044",
        "color #00f0ff",
        "color #ffe600",
        "color #ff00ff",
        "color #ffffff",
        "color #ff8800",
        "color #0088ff",
    ];
    for c in colors {
        items.push(SuggestionItem {
            text: c.to_string(),
            category: "Color",
        });
    }

    // 3. Quick Size / Thickness / Gap
    let dimensions = [
        "size 24",
        "size 32",
        "size 48",
        "thickness 2",
        "thickness 3",
        "thickness 4",
        "gap 4",
        "gap 6",
        "gap 8",
    ];
    for d in dimensions {
        items.push(SuggestionItem {
            text: d.to_string(),
            category: "Dimension",
        });
    }

    // 4. Actions
    let actions = [
        ("toggle", "Action"),
        ("reset", "Action"),
        ("status", "Action"),
        ("reload", "Action"),
        ("detach", "System"),
        ("quit", "System"),
    ];
    for (a, cat) in actions {
        items.push(SuggestionItem {
            text: a.to_string(),
            category: cat,
        });
    }

    // 5. All 200 Kenney SVGs
    for (k, _) in KENNEY_SVGS {
        items.push(SuggestionItem {
            text: format!("style kenney/{}", k),
            category: "Kenney SVG",
        });
    }

    items
}

pub fn get_all_style_keys() -> Vec<String> {
    let mut list = vec![
        "cross".to_string(),
        "dot".to_string(),
        "circle".to_string(),
        "circledot".to_string(),
        "chevron".to_string(),
        "box".to_string(),
        "tstyle".to_string(),
    ];

    for (k, _) in KENNEY_SVGS {
        list.push(format!("kenney/{}", k));
    }

    list
}

pub fn run_interactive() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, Hide)?;

    let mut current_config = match send_command(&Command::GetStatus) {
        Ok(Response::Status(cfg)) => cfg,
        _ => Config::load(),
    };

    let mut mode = Mode::Nudge;
    let mut input_buffer = String::new();
    let mut selected_idx: usize = 0;
    let mut status_msg = "Ready".to_string();

    let all_suggestions = get_all_suggestions();
    let initial_filtered = filter_suggestions(&input_buffer, &all_suggestions);
    render_dashboard(&current_config, &mode, &input_buffer, &status_msg, &initial_filtered, selected_idx)?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Ctrl+C: Quit
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    let _ = send_command(&Command::Quit);
                    break;
                }

                let filtered = filter_suggestions(&input_buffer, &all_suggestions);

                match mode {
                    Mode::Nudge => {
                        match key.code {
                            KeyCode::Tab => {
                                mode = Mode::Command;
                                input_buffer.clear();
                                selected_idx = 0;
                                status_msg = "Command Mode: Type & press Enter, Tab to return".to_string();
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                let _ = send_command(&Command::Quit);
                                break;
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                // Detach & keep running
                                break;
                            }
                            KeyCode::Char(' ') => {
                                let resp = send_command(&Command::Toggle);
                                if let Ok(Response::Status(cfg)) = resp {
                                    current_config = cfg;
                                } else {
                                    current_config.visible = !current_config.visible;
                                }
                                status_msg = if current_config.visible {
                                    "Overlay: Visible".to_string()
                                } else {
                                    "Overlay: Hidden".to_string()
                                };
                            }
                            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                                let _ = send_command(&Command::Nudge { dx: 0, dy: -1 });
                                current_config.offset_y -= 1;
                                status_msg = "Moved Up 1px".to_string();
                            }
                            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                                let _ = send_command(&Command::Nudge { dx: 0, dy: 1 });
                                current_config.offset_y += 1;
                                status_msg = "Moved Down 1px".to_string();
                            }
                            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                                let _ = send_command(&Command::Nudge { dx: -1, dy: 0 });
                                current_config.offset_x -= 1;
                                status_msg = "Moved Left 1px".to_string();
                            }
                            KeyCode::Right => {
                                let _ = send_command(&Command::Nudge { dx: 1, dy: 0 });
                                current_config.offset_x += 1;
                                status_msg = "Moved Right 1px".to_string();
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                let _ = send_command(&Command::ResetPosition);
                                current_config.offset_x = 0;
                                current_config.offset_y = 0;
                                status_msg = "Position Centered".to_string();
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                current_config.size = (current_config.size + 2).min(160);
                                let _ = send_command(&Command::SetSize(current_config.size));
                                status_msg = format!("Size: {}px", current_config.size);
                            }
                            KeyCode::Char('-') | KeyCode::Char('_') => {
                                current_config.size = (current_config.size.saturating_sub(2)).max(4);
                                let _ = send_command(&Command::SetSize(current_config.size));
                                status_msg = format!("Size: {}px", current_config.size);
                            }
                            KeyCode::Char('c') | KeyCode::Char('C') => {
                                let next_hex = if current_config.color[0] > 200 && current_config.color[1] < 100 {
                                    "#00f0ff"
                                } else if current_config.color[1] > 200 && current_config.color[0] < 100 {
                                    "#ff0044"
                                } else if current_config.color[2] > 200 && current_config.color[0] < 100 {
                                    "#ffe600"
                                } else {
                                    "#00ff88"
                                };
                                let _ = send_command(&Command::SetColor(next_hex.to_string()));
                                if let Ok(Response::Status(cfg)) = send_command(&Command::GetStatus) {
                                    current_config = cfg;
                                }
                                status_msg = format!("Color: {}", next_hex);
                            }
                            _ => {}
                        }
                    }
                    Mode::Command => {
                        match key.code {
                            KeyCode::Tab => {
                                // Tab switches exclusively between modes!
                                mode = Mode::Nudge;
                                input_buffer.clear();
                                selected_idx = 0;
                                status_msg = "Nudge Mode: Use Arrow Keys to Move".to_string();
                            }
                            KeyCode::Esc => {
                                mode = Mode::Nudge;
                                input_buffer.clear();
                                selected_idx = 0;
                                status_msg = "Nudge Mode: Use Arrow Keys to Move".to_string();
                            }
                            KeyCode::Up => {
                                if !filtered.is_empty() {
                                    selected_idx = if selected_idx == 0 {
                                        filtered.len() - 1
                                    } else {
                                        selected_idx - 1
                                    };
                                }
                            }
                            KeyCode::Down => {
                                if !filtered.is_empty() {
                                    selected_idx = (selected_idx + 1) % filtered.len();
                                }
                            }
                            KeyCode::Enter => {
                                // Enter accepts and executes the selected autocomplete item or typed text
                                let cmd_to_run = if !input_buffer.trim().is_empty() {
                                    if !filtered.is_empty() && selected_idx < filtered.len() && (input_buffer.len() < filtered[selected_idx].text.len()) {
                                        filtered[selected_idx].text.clone()
                                    } else {
                                        input_buffer.trim().to_string()
                                    }
                                } else if !filtered.is_empty() {
                                    filtered[selected_idx % filtered.len()].text.clone()
                                } else {
                                    String::new()
                                };

                                if !cmd_to_run.is_empty() {
                                    status_msg = execute_typed_command(&cmd_to_run, &mut current_config);
                                    if cmd_to_run == "quit" || cmd_to_run == "q" || cmd_to_run == "exit" || cmd_to_run == "detach" || cmd_to_run == "d" {
                                        break;
                                    }
                                    input_buffer.clear();
                                    selected_idx = 0;
                                }
                            }
                            KeyCode::Backspace => {
                                input_buffer.pop();
                                selected_idx = 0;
                            }
                            KeyCode::Char(c) => {
                                input_buffer.push(c);
                                selected_idx = 0;
                            }
                            _ => {}
                        }
                    }
                }

                let final_filtered = filter_suggestions(&input_buffer, &all_suggestions);
                render_dashboard(&current_config, &mode, &input_buffer, &status_msg, &final_filtered, selected_idx)?;
            }
        }
    }

    execute!(out, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    println!("CrossOver session closed.");
    Ok(())
}

fn filter_suggestions<'a>(input: &str, items: &'a [SuggestionItem]) -> Vec<&'a SuggestionItem> {
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        return items.iter().take(25).collect();
    }

    let mut matched = Vec::new();
    for item in items {
        if item.text.to_lowercase().contains(&trimmed) {
            matched.push(item);
        }
    }
    matched
}

fn execute_typed_command(cmd: &str, config: &mut Config) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return "Empty command".to_string();
    }

    match parts[0].to_lowercase().as_str() {
        "style" | "s" => {
            if let Some(name) = parts.get(1) {
                let _ = send_command(&Command::SetStyle(name.to_string()));
                if let Ok(Response::Status(cfg)) = send_command(&Command::GetStatus) {
                    *config = cfg;
                }
                format!("Style set to: {}", name)
            } else {
                "Usage: style <name>".to_string()
            }
        }
        "color" | "c" => {
            if let Some(hex) = parts.get(1) {
                let _ = send_command(&Command::SetColor(hex.to_string()));
                if let Ok(Response::Status(cfg)) = send_command(&Command::GetStatus) {
                    *config = cfg;
                }
                format!("Color set to: {}", hex)
            } else {
                "Usage: color <hex>".to_string()
            }
        }
        "size" | "sz" => {
            if let Some(Ok(s)) = parts.get(1).map(|v| v.parse::<u32>()) {
                let _ = send_command(&Command::SetSize(s));
                config.size = s;
                format!("Size set to: {}px", s)
            } else {
                "Usage: size <px>".to_string()
            }
        }
        "thickness" | "t" => {
            if let Some(Ok(t)) = parts.get(1).map(|v| v.parse::<u32>()) {
                let _ = send_command(&Command::SetThickness(t));
                config.thickness = t;
                format!("Thickness set to: {}px", t)
            } else {
                "Usage: thickness <px>".to_string()
            }
        }
        "gap" | "g" => {
            if let Some(Ok(g)) = parts.get(1).map(|v| v.parse::<u32>()) {
                let _ = send_command(&Command::SetGap(g));
                config.gap = g;
                format!("Gap set to: {}px", g)
            } else {
                "Usage: gap <px>".to_string()
            }
        }
        "toggle" => {
            let resp = send_command(&Command::Toggle);
            if let Ok(Response::Status(cfg)) = resp {
                *config = cfg;
            }
            format!("Overlay: {}", if config.visible { "Visible" } else { "Hidden" })
        }
        "reset" | "center" => {
            let _ = send_command(&Command::ResetPosition);
            config.offset_x = 0;
            config.offset_y = 0;
            "Position reset to center".to_string()
        }
        "quit" | "q" | "exit" => {
            let _ = send_command(&Command::Quit);
            "Quitting...".to_string()
        }
        "detach" | "d" => "Detached".to_string(),
        other => {
            let _ = send_command(&Command::SetStyle(other.to_string()));
            if let Ok(Response::Status(cfg)) = send_command(&Command::GetStatus) {
                *config = cfg;
                format!("Style set to: {}", other)
            } else {
                format!("Executed: {}", other)
            }
        }
    }
}

fn render_dashboard(
    cfg: &Config,
    mode: &Mode,
    input_buffer: &str,
    status_msg: &str,
    filtered: &[&SuggestionItem],
    selected_idx: usize,
) -> Result<(), std::io::Error> {
    let mut out = stdout();
    execute!(out, MoveTo(0, 0), Clear(ClearType::All))?;

    let vis_str = if cfg.visible {
        "🟢 ON"
    } else {
        "🔴 OFF"
    };
    let hex_color = format!(
        "#{:02X}{:02X}{:02X}",
        cfg.color[0], cfg.color[1], cfg.color[2]
    );

    let (mode_badge, mode_color) = match mode {
        Mode::Nudge => ("🎮 [MODE: NUDGE - Arrow Keys Move]", Color::Green),
        Mode::Command => ("⌨️ [MODE: COMMAND - VSCode Autocomplete]", Color::Yellow),
    };

    execute!(
        out,
        SetForegroundColor(Color::Cyan),
        Print("╔══════════════════════════════════════════════════════════════════════╗\r\n"),
        Print("║                       🎯 CROSSOVER CONTROLLER                        ║\r\n"),
        Print("╠══════════════════════════════════════════════════════════════════════╣\r\n"),
        ResetColor,
        Print(format!("║  Status: {:<6}  Style: {:<20}  Size: {:<3}px  Gap: {:<3}px  ║\r\n", vis_str, cfg.style.to_string_repr(), cfg.size, cfg.gap)),
        Print(format!("║  Color:  {:<7}  Thickness: {:<2}px   Position: [X: {:<3}, Y: {:<3}]            ║\r\n", hex_color, cfg.thickness, cfg.offset_x, cfg.offset_y)),
        SetForegroundColor(Color::Cyan),
        Print("╠══════════════════════════════════════════════════════════════════════╣\r\n"),
        SetForegroundColor(mode_color),
        Print(format!("║  {:<68}║\r\n", mode_badge)),
        ResetColor,
    )?;

    match mode {
        Mode::Nudge => {
            execute!(
                out,
                SetForegroundColor(Color::White),
                Print("║  [↑ ↓ ← → / WASD] Move 1px       [Space] Toggle Show/Hide            ║\r\n"),
                Print("║  [+ / -] Adjust Size             [C] Cycle Color      [R] Center     ║\r\n"),
                Print("║  [D] Detach & Run in Background  [Q] Quit Overlay                    ║\r\n"),
                SetForegroundColor(Color::Cyan),
                Print("║  👉 Press [Tab] to switch to Command Mode & VSCode Autocomplete      ║\r\n"),
                Print("╠══════════════════════════════════════════════════════════════════════╣\r\n"),
                ResetColor,
            )?;
        }
        Mode::Command => {
            // Input prompt
            execute!(
                out,
                Print("║  > "),
                SetForegroundColor(Color::Yellow),
                Print(input_buffer),
                SetForegroundColor(Color::White),
                Print("█"),
                ResetColor,
            )?;
            let total_len = 5 + input_buffer.len() + 1;
            let spaces = 70usize.saturating_sub(total_len);
            execute!(out, Print(" ".repeat(spaces)), Print("║\r\n"))?;

            // Autocomplete box header
            execute!(
                out,
                SetForegroundColor(Color::DarkGrey),
                Print("║  ┌──────────────────────────────────────────────────────────────┐    ║\r\n"),
                ResetColor,
            )?;

            // Render up to 5 suggestions
            let max_visible = 5;
            let start = if filtered.len() <= max_visible || selected_idx < max_visible {
                0
            } else {
                selected_idx.saturating_sub(max_visible - 1)
            };
            let end = (start + max_visible).min(filtered.len());

            for (i, item) in filtered.iter().enumerate().take(end).skip(start) {
                let is_selected = i == selected_idx;
                execute!(out, Print("║  │ "))?;

                if is_selected {
                    execute!(
                        out,
                        SetForegroundColor(Color::Black),
                        SetBackgroundColor(Color::Cyan),
                        Print(format!(" ► {:<38} {:>17} ", item.text, format!("[{}]", item.category))),
                        ResetColor,
                    )?;
                } else {
                    execute!(
                        out,
                        SetForegroundColor(Color::White),
                        Print(format!("   {:<38} ", item.text)),
                        SetForegroundColor(Color::DarkGrey),
                        Print(format!("{:>17} ", format!("[{}]", item.category))),
                        ResetColor,
                    )?;
                }
                execute!(out, Print(" │    ║\r\n"))?;
            }

            if filtered.is_empty() {
                execute!(
                    out,
                    Print("║  │ "),
                    SetForegroundColor(Color::DarkGrey),
                    Print("   (No matching commands or styles)                       "),
                    ResetColor,
                    Print(" │    ║\r\n"),
                )?;
            }

            // Autocomplete box footer
            execute!(
                out,
                SetForegroundColor(Color::DarkGrey),
                Print("║  └──────────────────────────────────────────────────────────────┘    ║\r\n"),
                SetForegroundColor(Color::Cyan),
                Print("║  [↑/↓] Select   [Enter] Accept & Execute   [Tab/Esc] Nudge Mode      ║\r\n"),
                Print("╠══════════════════════════════════════════════════════════════════════╣\r\n"),
                ResetColor,
            )?;
        }
    }

    // Message bar
    execute!(
        out,
        SetForegroundColor(Color::Magenta),
        Print(format!("║  🔔 Info: {:<58}║\r\n", status_msg)),
        SetForegroundColor(Color::Cyan),
        Print("╚══════════════════════════════════════════════════════════════════════╝\r\n"),
        ResetColor,
    )?;

    out.flush()?;
    Ok(())
}
