//! Обработка ввода: клавиатура (WASD, пробел, Shift) и мышь (поворот камеры).

use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::Player;

/// Состояние клавиш движения.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sneak: bool,
}

/// Обработчик ввода: обновляет InputState и вращение игрока.
pub struct InputHandler {
    pub state: InputState,
    pub mouse_captured: bool,
    pub last_mouse_pos: Option<(f64, f64)>,
    pub mouse_sensitivity: f32,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
            mouse_captured: false,
            last_mouse_pos: None,
            mouse_sensitivity: 0.15,
        }
    }

    /// Обработать событие окна. Возвращает true, если событие было обработано.
    pub fn handle_event(&mut self, event: &WindowEvent, player: &mut Player) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(key),
                        ..
                    },
                ..
            } => {
                let pressed = *state == ElementState::Pressed;
                match key {
                    KeyCode::KeyW => self.state.forward = pressed,
                    KeyCode::KeyS => self.state.backward = pressed,
                    KeyCode::KeyA => self.state.left = pressed,
                    KeyCode::KeyD => self.state.right = pressed,
                    KeyCode::Space => self.state.jump = pressed,
                    KeyCode::ShiftLeft => self.state.sneak = pressed,
                    KeyCode::Escape => {
                        if pressed {
                            self.mouse_captured = false;
                        }
                    }
                    _ => return false,
                }
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                // ЛКМ — ломание блока (обрабатывается отдельно в block_interaction)
                self.mouse_captured = true;
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Right,
                state: ElementState::Pressed,
                ..
            } => {
                // ПКМ — установка блока
                self.mouse_captured = true;
                true
            }
            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                if self.mouse_captured {
                    if let Some((last_x, last_y)) = self.last_mouse_pos {
                        let dx = (position.x - last_x) as f32 * self.mouse_sensitivity;
                        let dy = (position.y - last_y) as f32 * self.mouse_sensitivity;
                        player.rotate(dx, -dy); // инвертируем Y
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                } else {
                    self.last_mouse_pos = Some((position.x, position.y));
                }
                true
            }
            _ => false,
        }
    }

    /// Получить вектор движения в плоскости XZ на основе состояния клавиш и угла игрока.
    pub fn movement_vector(&self, player: &Player) -> glam::Vec3 {
        let mut move_vec = glam::Vec3::ZERO;
        if self.state.forward {
            move_vec += player.forward_flat();
        }
        if self.state.backward {
            move_vec -= player.forward_flat();
        }
        if self.state.right {
            move_vec += player.right_flat();
        }
        if self.state.left {
            move_vec -= player.right_flat();
        }
        move_vec
    }
}
