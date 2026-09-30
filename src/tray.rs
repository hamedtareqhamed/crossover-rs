use crate::config::{Config, CrosshairStyle};
use crate::ipc::{self, Command, Response};
use crate::renderer::Renderer;
use ksni::blocking::TrayMethods;
use ksni::menu::*;
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray};
use std::env;

pub struct CrosshairTray {
    pub config: Config,
}

impl CrosshairTray {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn update_from_daemon(&mut self) {
        if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::GetStatus) {
            self.config = cfg;
        }
    }

    fn render_icon(&self) -> Vec<Icon> {
        let size: u32 = 32;
        let mut preview_config = self.config.clone();
        preview_config.visible = true;
        preview_config.offset_x = 0;
        preview_config.offset_y = 0;
        preview_config.size = 22;

        if let Some(pixmap) = Renderer::render(&preview_config, size) {
            let mut argb_data = Vec::with_capacity((size * size * 4) as usize);
            for chunk in pixmap.data().chunks_exact(4) {
                // RGBA to ARGB network byte order
                argb_data.push(chunk[3]); // Alpha
                argb_data.push(chunk[0]); // Red
                argb_data.push(chunk[1]); // Green
                argb_data.push(chunk[2]); // Blue
            }
            vec![Icon {
                width: size as i32,
                height: size as i32,
                data: argb_data,
            }]
        } else {
            Vec::new()
        }
    }
}

impl Tray for CrosshairTray {
    fn id(&self) -> String {
        "crossover-overlay".into()
    }

    fn title(&self) -> String {
        "CrossOver".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn icon_name(&self) -> String {
        "crossover".into()
    }

    fn icon_theme_path(&self) -> String {
        if let Ok(home) = env::var("HOME") {
            format!("{}/.local/share/icons/hicolor/scalable/apps", home)
        } else {
            String::new()
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.render_icon()
    }

    fn tool_tip(&self) -> ToolTip {
        let status_text = if self.config.visible { "Visible" } else { "Hidden" };
        ToolTip {
            icon_name: "crossover".into(),
            icon_pixmap: self.render_icon(),
            title: "CrossOver Crosshair".into(),
            description: format!(
                "Status: {}\nStyle: {}\nPosition: ({:+}, {:+})\nSize: {}px",
                status_text,
                self.config.style.to_string_repr(),
                self.config.offset_x,
                self.config.offset_y,
                self.config.size
            ),
        }
    }

    // Left-click on tray icon toggles crosshair visibility instantly
    fn activate(&mut self, _x: i32, _y: i32) {
        if let Ok(Response::Status(updated)) = ipc::send_command(&Command::Toggle) {
            self.config = updated;
        } else {
            self.config.visible = !self.config.visible;
        }
    }

    fn menu_about_to_show(&mut self) {
        self.update_from_daemon();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = Vec::new();

        // 1. Visibility Toggle
        items.push(
            CheckmarkItem {
                label: if self.config.visible {
                    "👁️  Crosshair: Visible".into()
                } else {
                    "👁️  Crosshair: Hidden".into()
                },
                checked: self.config.visible,
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Toggle) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // 2. Position Submenu (الطريقة المباشرة - Direct Method)
        let pos_label = format!("📍 Position: ({:+}, {:+})", self.config.offset_x, self.config.offset_y);
        let mut pos_menu = Vec::new();

        pos_menu.push(
            StandardItem {
                label: format!("Current: X: {:+}, Y: {:+}", self.config.offset_x, self.config.offset_y),
                enabled: false,
                ..Default::default()
            }
            .into(),
        );

        pos_menu.push(MenuItem::Separator);

        // Up
        pos_menu.push(
            StandardItem {
                label: "⬆️  Up (+1 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 0, dy: -1 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        pos_menu.push(
            StandardItem {
                label: "⬆️  Up (+5 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 0, dy: -5 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        // Down
        pos_menu.push(
            StandardItem {
                label: "⬇️  Down (+1 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 0, dy: 1 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        pos_menu.push(
            StandardItem {
                label: "⬇️  Down (+5 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 0, dy: 5 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        // Left
        pos_menu.push(
            StandardItem {
                label: "⬅️  Left (+1 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: -1, dy: 0 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        pos_menu.push(
            StandardItem {
                label: "⬅️  Left (+5 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: -5, dy: 0 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        // Right
        pos_menu.push(
            StandardItem {
                label: "➡️  Right (+1 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 1, dy: 0 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );
        pos_menu.push(
            StandardItem {
                label: "➡️  Right (+5 px)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::Nudge { dx: 5, dy: 0 }) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        pos_menu.push(MenuItem::Separator);

        // Reset
        pos_menu.push(
            StandardItem {
                label: "🎯 Reset to Center (0, 0)".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::ResetPosition) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        items.push(
            SubMenu {
                label: pos_label,
                submenu: pos_menu,
                ..Default::default()
            }
            .into(),
        );

        // 3. Styles Submenu
        let styles = [
            ("Cross", "cross", CrosshairStyle::Cross),
            ("Dot", "dot", CrosshairStyle::Dot),
            ("Circle", "circle", CrosshairStyle::Circle),
            ("Circle Dot", "circledot", CrosshairStyle::CircleDot),
            ("Chevron", "chevron", CrosshairStyle::Chevron),
            ("Box", "box", CrosshairStyle::Box),
            ("T-Style", "tstyle", CrosshairStyle::TStyle),
        ];

        let mut style_menu = Vec::new();
        for (label, key, style_enum) in styles {
            let is_checked = self.config.style == style_enum;
            let key_str = key.to_string();
            style_menu.push(
                CheckmarkItem {
                    label: label.into(),
                    checked: is_checked,
                    activate: Box::new(move |this: &mut Self| {
                        if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetStyle(key_str.clone())) {
                            this.config = cfg;
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(
            SubMenu {
                label: format!("🎯 Style: {}", self.config.style.to_string_repr()),
                submenu: style_menu,
                ..Default::default()
            }
            .into(),
        );

        // 4. Color Submenu
        let colors = [
            ("🟢 Neon Green", "#00ff88", [0, 255, 136]),
            ("🔴 Crimson Red", "#ff3366", [255, 51, 102]),
            ("🔵 Cyan Blue", "#00e5ff", [0, 229, 255]),
            ("🟡 Vibrant Yellow", "#ffee00", [255, 238, 0]),
            ("🟣 Electric Purple", "#ff00ff", [255, 0, 255]),
            ("⚪ Pure White", "#ffffff", [255, 255, 255]),
        ];

        let mut color_menu = Vec::new();
        for (label, hex, rgb) in colors {
            let is_checked = self.config.color[0] == rgb[0]
                && self.config.color[1] == rgb[1]
                && self.config.color[2] == rgb[2];
            let hex_str = hex.to_string();
            color_menu.push(
                CheckmarkItem {
                    label: label.into(),
                    checked: is_checked,
                    activate: Box::new(move |this: &mut Self| {
                        if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetColor(hex_str.clone())) {
                            this.config = cfg;
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(
            SubMenu {
                label: "🎨 Color".into(),
                submenu: color_menu,
                ..Default::default()
            }
            .into(),
        );

        // 5. Size Submenu
        let mut size_menu = Vec::new();
        let cur_size = self.config.size;

        size_menu.push(
            StandardItem {
                label: "➕ Larger (+4 px)".into(),
                activate: Box::new(move |this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetSize(cur_size + 4)) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        size_menu.push(
            StandardItem {
                label: "➖ Smaller (-4 px)".into(),
                activate: Box::new(move |this: &mut Self| {
                    let new_sz = cur_size.saturating_sub(4).max(4);
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetSize(new_sz)) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        size_menu.push(MenuItem::Separator);

        for (preset_name, sz) in [
            ("Small (20 px)", 20),
            ("Medium (28 px)", 28),
            ("Large (36 px)", 36),
            ("Extra Large (48 px)", 48),
        ] {
            size_menu.push(
                CheckmarkItem {
                    label: preset_name.into(),
                    checked: self.config.size == sz,
                    activate: Box::new(move |this: &mut Self| {
                        if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetSize(sz)) {
                            this.config = cfg;
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(
            SubMenu {
                label: format!("📏 Size: {} px", self.config.size),
                submenu: size_menu,
                ..Default::default()
            }
            .into(),
        );

        // 6. Thickness Submenu
        let mut thick_menu = Vec::new();
        let cur_th = self.config.thickness;

        thick_menu.push(
            StandardItem {
                label: "➕ Thicker (+1 px)".into(),
                activate: Box::new(move |this: &mut Self| {
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetThickness(cur_th + 1)) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        thick_menu.push(
            StandardItem {
                label: "➖ Thinner (-1 px)".into(),
                activate: Box::new(move |this: &mut Self| {
                    let new_th = cur_th.saturating_sub(1).max(1);
                    if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetThickness(new_th)) {
                        this.config = cfg;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        thick_menu.push(MenuItem::Separator);

        for th in 1..=4 {
            thick_menu.push(
                CheckmarkItem {
                    label: format!("{} px", th),
                    checked: self.config.thickness == th,
                    activate: Box::new(move |this: &mut Self| {
                        if let Ok(Response::Status(cfg)) = ipc::send_command(&Command::SetThickness(th)) {
                            this.config = cfg;
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(
            SubMenu {
                label: format!("⚡ Thickness: {} px", self.config.thickness),
                submenu: thick_menu,
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // 7. Quit Application
        items.push(
            StandardItem {
                label: "❌ Quit CrossOver".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_this: &mut Self| {
                    let _ = ipc::send_command(&Command::Quit);
                    std::process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

pub fn spawn_tray(config: Config) {
    std::thread::spawn(move || {
        // Wait briefly for IPC socket to be ready
        std::thread::sleep(std::time::Duration::from_millis(150));
        let tray = CrosshairTray::new(config);
        match tray.spawn() {
            Ok(_handle) => {
                // Keep tray thread alive
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                eprintln!("[crossover] Warning: Failed to spawn System Tray: {}", e);
            }
        }
    });
}
