//! Движение игрока с коллизиями против блоков.
//! Реализует простую физику: ускорение, трение, гравитацию, прыжок, столкновение с AABB блоков.

use crate::input::InputHandler;
use crate::Player;

/// Константы физики игрока.
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_WIDTH_HALF: f32 = PLAYER_WIDTH / 2.0;
pub const GRAVITY: f32 = -20.0;
pub const JUMP_SPEED: f32 = 6.5;
pub const MOVE_SPEED: f32 = 4.0;
pub const AIR_CONTROL: f32 = 0.2;
pub const FRICTION: f32 = 0.8;
pub const GROUND_FRICTION: f32 = 0.4;

/// Применить ввод и физику к игроку за один кадр.
/// `dt` — время в секундах с прошлого кадра.
/// `world` — доступ к миру для проверки коллизий.
pub fn apply_movement(
    player: &mut Player,
    input: &InputHandler,
    world: &crate::WorldContext,
    dt: f32,
) {
    let move_vec = input.movement_vector(player);

    // Ускорение
    let mut accel = move_vec.normalize_or_zero() * MOVE_SPEED;

    // Вертикальная скорость (гравитация)
    player.velocity.y += GRAVITY * dt;

    // Прыжок
    if input.state.jump && player.on_ground {
        player.velocity.y = JUMP_SPEED;
        player.on_ground = false;
    }

    // Применяем ускорение в зависимости от того, в воздухе или на земле
    if player.on_ground {
        accel.y = 0.0;
        player.velocity.x += accel.x * dt;
        player.velocity.z += accel.z * dt;
        player.velocity.x *= GROUND_FRICTION.powf(dt * 10.0);
        player.velocity.z *= GROUND_FRICTION.powf(dt * 10.0);
    } else {
        player.velocity.x += accel.x * AIR_CONTROL * dt;
        player.velocity.z += accel.z * AIR_CONTROL * dt;
    }

    // Ограничиваем скорость по горизонтали
    let max_speed = if player.on_ground { MOVE_SPEED } else { 10.0 };
    let horizontal_speed = (player.velocity.x * player.velocity.x + player.velocity.z * player.velocity.z).sqrt();
    if horizontal_speed > max_speed {
        let scale = max_speed / horizontal_speed;
        player.velocity.x *= scale;
        player.velocity.z *= scale;
    }

    // Перемещаем игрока с коллизиями по осям отдельно
    move_with_collisions(player, world, dt);

    // Обновляем флаг на земле (проверяем, есть ли блок под ногами)
    player.on_ground = is_on_ground(player, world);
}

/// Проверка, находится ли игрок на земле (блок под ногами).
fn is_on_ground(player: &Player, world: &crate::WorldContext) -> bool {
    let foot_y = (player.position.y - PLAYER_HEIGHT / 2.0 - 0.01).floor() as i32;
    for dx in -1..=1i32 {
        for dz in -1..=1i32 {
            let wx = (player.position.x + dx as f32 * PLAYER_WIDTH_HALF).floor() as i32;
            let wz = (player.position.z + dz as f32 * PLAYER_WIDTH_HALF).floor() as i32;
            if let Some(block) = world.get_block(wx, foot_y, wz) {
                if block.path() != "air" {
                    return true;
                }
            }
        }
    }
    false
}

/// Переместить игрока с коллизиями против блоков (раздельно по осям).
fn move_with_collisions(player: &mut Player, world: &crate::WorldContext, dt: f32) {
    let delta = player.velocity * dt;

    // Ось X
    player.position.x += delta.x;
    if collides_with_world(player, world) {
        player.position.x -= delta.x;
        player.velocity.x = 0.0;
    }

    // Ось Y
    player.position.y += delta.y;
    if collides_with_world(player, world) {
        player.position.y -= delta.y;
        if delta.y > 0.0 {
            player.velocity.y = 0.0; // ударились головой
        } else {
            player.velocity.y = 0.0; // приземлились
            player.on_ground = true;
        }
    }

    // Ось Z
    player.position.z += delta.z;
    if collides_with_world(player, world) {
        player.position.z -= delta.z;
        player.velocity.z = 0.0;
    }
}

/// Проверка коллизии игрока с блоками мира (AABB).
fn collides_with_world(player: &Player, world: &crate::WorldContext) -> bool {
    let min_x = (player.position.x - PLAYER_WIDTH_HALF).floor() as i32;
    let max_x = (player.position.x + PLAYER_WIDTH_HALF).floor() as i32;
    let min_y = (player.position.y - PLAYER_HEIGHT / 2.0).floor() as i32;
    let max_y = (player.position.y + PLAYER_HEIGHT / 2.0).floor() as i32;
    let min_z = (player.position.z - PLAYER_WIDTH_HALF).floor() as i32;
    let max_z = (player.position.z + PLAYER_WIDTH_HALF).floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                if let Some(block) = world.get_block(x, y, z) {
                    if block.path() != "air" {
                        return true;
                    }
                }
            }
        }
    }
    false
}
