// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Простое immediate-mode GUI поверх мира.
//! Рисование цветных прямоугольников через wgpu, без внешних зависимостей.
//! Используется для главного меню, экранов 싱글плеера/мультиплеера и настроек.

use winit::event::{ElementState, MouseButton, WindowEvent};

/// Состояние главного меню.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuScreen {
    #[default]
    None,
    Main,
    Singleplayer,
    Multiplayer,
    Settings,
}

/// Кнопка на экране.
#[derive(Debug, Clone)]
pub struct Button {
    pub id: &'static str,
    pub label: &'static str,
    pub rect: Rect,
}

/// Прямоугольник в нормализованных координатах (0.0..1.0).
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn center(w: f32, h: f32) -> Self {
        Self { x: 0.5 - w / 2.0, y: 0.5 - h / 2.0, w, h }
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }
}

/// Состояние UI: нажата ли кнопка, координаты мыши.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub buttons: Vec<Button>,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_pressed: bool,
    pub screen: MenuScreen,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            mouse_x: 0.5,
            mouse_y: 0.5,
            mouse_pressed: false,
            screen: MenuScreen::None,
        }
    }

    pub fn main_menu() -> Self {
        let mut s = Self::new();
        s.screen = MenuScreen::Main;
        s.buttons = vec![
            Button { id: "singleplayer", label: "Одиночная игра", rect: Rect::center(0.3, 0.08) },
            Button { id: "multiplayer",  label: "Мультиплеер",      rect: Rect { x: 0.35, y: 0.42, w: 0.3, h: 0.08 } },
            Button { id: "settings",     label: "Настройки",        rect: Rect { x: 0.35, y: 0.54, w: 0.3, h: 0.08 } },
            Button { id: "quit",         label: "Выход",            rect: Rect { x: 0.35, y: 0.66, w: 0.3, h: 0.08 } },
        ];
        // Сдвинем первую кнопку вверх
        s.buttons[0].rect = Rect { x: 0.35, y: 0.30, w: 0.3, h: 0.08 };
        s
    }

    pub fn singleplayer() -> Self {
        let mut s = Self::new();
        s.screen = MenuScreen::Singleplayer;
        s.buttons = vec![
            Button { id: "back", label: "Назад", rect: Rect { x: 0.35, y: 0.80, w: 0.3, h: 0.08 } },
        ];
        s
    }

    pub fn multiplayer() -> Self {
        let mut s = Self::new();
        s.screen = MenuScreen::Multiplayer;
        s.buttons = vec![
            Button { id: "add_server", label: "Добавить сервер", rect: Rect { x: 0.35, y: 0.40, w: 0.3, h: 0.08 } },
            Button { id: "back",       label: "Назад",           rect: Rect { x: 0.35, y: 0.80, w: 0.3, h: 0.08 } },
        ];
        s
    }

    pub fn settings() -> Self {
        let mut s = Self::new();
        s.screen = MenuScreen::Settings;
        s.buttons = vec![
            Button { id: "back", label: "Назад", rect: Rect { x: 0.35, y: 0.80, w: 0.3, h: 0.08 } },
        ];
        s
    }

    /// Обновить позицию мыши и проверить нажатие.
    pub fn update_mouse(&mut self, mx: f32, my: f32, pressed: bool) -> Option<&'static str> {
        self.mouse_x = mx;
        self.mouse_y = my;
        self.mouse_pressed = pressed;
        if pressed {
            for btn in &self.buttons {
                if btn.rect.contains(mx, my) {
                    return Some(btn.id);
                }
            }
        }
        None
    }
}

/// Отрисовать UI поверх кадра.
/// `render_pass` — активный проход, `width`/`height` — размеры окна.
pub fn render_ui(
    render_pass: &mut wgpu::RenderPass,
    ui: &UiState,
    width: u32,
    height: u32,
) {
    // В реальном коде здесь были бы вершинные буферы с прямоугольниками.
    // Пока заполним заглушку: просто используем существующий конвейер мира.
    let _ = (render_pass, ui, width, height);
}

/// Обработать событие окна для UI.
pub fn handle_event(ui: &mut UiState, event: &WindowEvent, width: u32, height: u32) -> Option<&'static str> {
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let mx = position.x as f32 / width as f32;
            let my = position.y as f32 / height as f32;
            ui.update_mouse(mx, my, false)
        }
        WindowEvent::MouseInput { button: MouseButton::Left, state: ElementState::Pressed, .. } => {
            ui.update_mouse(ui.mouse_x, ui.mouse_y, true)
        }
        _ => None,
    }
}
