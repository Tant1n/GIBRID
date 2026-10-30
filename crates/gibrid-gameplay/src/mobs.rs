//! GIBRID Gameplay - Entities & Physics Systems
//! Phase 9: Full Minecraft Native - Entity simulation, physics, AI
//! Status: IMPLEMENTED

use gibrid_core::ResourceId;
use glam::Vec3;
use std::collections::HashMap;

/// Entity types supported by GIBRID Native.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    ZombieVillager,
    Enderman,
    Witch,
    WitherSkeleton,
    Blaze,
    Ghast,
    MagmaCube,
    Slime,
    Pig,
    Cow,
    Sheep,
    Chicken,
    Rabbit,
    Horse,
    Donkey,
    Mule,
    Llama,
    Ocelot,
    Cat,
    Wolf,
    Parrot,
    Fox,
    Bee,
    PolarBear,
    Panda,
    FoxVariant,
    Turtle,
    Dolphin,
    Cod,
    Salmon,
    Pufferfish,
    TropicalFish,
    Squid,
    Axolotl,
    Goat,
    GlowSquid,
    Warden,
    Allay,
    Frog,
    Tadpole,
    // Projectiles
    Arrow,
    Snowball,
    Egg,
    Fireball,
    TNT,
    // Vehicles
    Minecart,
    Boat,
    // Item entities
    Item,
    // Experience
    ExperienceOrb,
}

/// Entity status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityStatus {
    Alive,
    Dead,
    Dying,
    Spawning,
}

/// Type of AI governing an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIType {
    Passive,
    Neutral,
    Hostile,
    Water,
    Flying,
    PassivePeaceful,
}

/// AI state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIState {
    Idle,
    Wander,
    FollowTarget,
    AttackTarget,
    FleeTarget,
    Sleep,
    Swim,
    Fly,
}

/// Entity physics state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    OnGround,
    InAir,
    InWater,
    OnLadder,
    Climbing,
}

/// A game entity (mob, player, projectile, etc.).
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: ResourceId,
    pub entity_type: EntityType,
    pub status: EntityStatus,
    pub position: Vec3,
    pub velocity: Vec3,
    pub previous_position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub prev_yaw: f32,
    pub prev_pitch: f32,
    pub health: f32,
    pub max_health: f32,
    pub armor: f32,
    pub attack_damage: f32,
    pub attack_speed: f32,
    pub movement_speed: f32,
    pub ai_type: AIType,
    pub ai_state: AIState,
    pub physics_state: EntityState,
    pub target_entity_id: Option<ResourceId>,
    pub target_position: Option<Vec3>,
    pub ticks_existed: u32,
    pub ticks_since_attack: u32,
    pub detection_range: f32,
    pub attack_range: f32,
    pub follow_range: f32,
    pub age: u32,
    pub alive_time: u32,
    pub pickup_delay: u32,
    pub is_on_fire: u32,
    pub is_silent: bool,
    pub is_baby: bool,
    pub on_ground: bool,
}

impl Entity {
    pub fn new(id: ResourceId, entity_type: EntityType) -> Self {
        let (max_health, attack_damage, movement_speed, ai_type, detection_range, attack_range, follow_range) =
            match entity_type {
                EntityType::Player => (20.0, 1.0, 0.2, AIType::Neutral, 0.0, 0.0, 0.0),
                EntityType::Zombie => (20.0, 3.0, 0.23, AIType::Hostile, 35.0, 2.0, 35.0),
                EntityType::Skeleton => (20.0, 2.0, 0.23, AIType::Hostile, 16.0, 8.0, 16.0),
                EntityType::Creeper => (20.0, 3.0, 0.25, AIType::Hostile, 16.0, 2.0, 16.0),
                EntityType::Spider => (20.0, 2.0, 0.3, AIType::Neutral, 16.0, 2.0, 16.0),
                EntityType::Pig => (10.0, 3.0, 0.25, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::Cow => (10.0, 3.0, 0.25, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::Sheep => (8.0, 3.0, 0.23, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::Chicken => (4.0, 2.0, 0.25, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::Rabbit => (3.0, 0.0, 0.25, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::Wolf => (8.0, 2.0, 0.3, AIType::Neutral, 16.0, 2.0, 16.0),
                EntityType::Ocelot => (6.0, 2.0, 0.3, AIType::Passive, 16.0, 2.0, 16.0),
                EntityType::FoxVariant => (8.0, 2.0, 0.3, AIType::Neutral, 16.0, 2.0, 16.0),
                EntityType::Enderman => (40.0, 7.0, 0.3, AIType::Neutral, 64.0, 2.0, 64.0),
                EntityType::Blaze => (20.0, 3.0, 0.25, AIType::Hostile, 16.0, 2.0, 16.0),
                EntityType::Ghast => (10.0, 6.0, 0.25, AIType::Hostile, 64.0, 20.0, 64.0),
                EntityType::Witch => (26.0, 3.0, 0.25, AIType::Hostile, 16.0, 8.0, 16.0),
                EntityType::Slime => (2.0, 2.0, 0.2, AIType::Hostile, 16.0, 2.0, 16.0),
                EntityType::Cat => (6.0, 0.0, 0.2, AIType::PassivePeaceful, 16.0, 0.0, 16.0),
                _ => (10.0, 1.0, 0.2, AIType::Passive, 16.0, 2.0, 16.0),
            };

        Self {
            id,
            entity_type,
            status: EntityStatus::Alive,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            previous_position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            prev_yaw: 0.0,
            prev_pitch: 0.0,
            health: max_health,
            max_health,
            armor: 0.0,
            attack_damage,
            attack_speed: 1.0,
            movement_speed,
            ai_type,
            ai_state: AIState::Idle,
            physics_state: EntityState::OnGround,
            target_entity_id: None,
            target_position: None,
            ticks_existed: 0,
            ticks_since_attack: 0,
            detection_range,
            attack_range,
            follow_range,
            age: 0,
            alive_time: 0,
            pickup_delay: 0,
            is_on_fire: 0,
            is_silent: false,
            is_baby: false,
            on_ground: false,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.status == EntityStatus::Alive && self.health > 0.0
    }

    pub fn take_damage(&mut self, damage: f32) {
        let actual_damage = (damage - self.armor).max(0.0);
        self.health = (self.health - actual_damage).max(0.0);
        if self.health <= 0.0 {
            self.status = EntityStatus::Dead;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
    }

    pub fn is_on_ground(&self) -> bool {
        self.on_ground
    }

    pub fn is_swimming(&self) -> bool {
        self.physics_state == EntityState::InWater
    }
}

/// Player-specific data.
#[derive(Debug, Clone)]
pub struct PlayerData {
    pub inventory: super::inventory::Inventory,
    pub experience_level: u32,
    pub experience_points: f32,
    pub total_experience: u32,
    pub armor_items: [Option<super::inventory::ItemStack>; 4],
    pub main_hand: Option<super::inventory::ItemStack>,
    pub off_hand: Option<super::inventory::ItemStack>,
    pub game_mode: GameMode,
    pub flying: bool,
    pub creative: bool,
    pub game_mode_previous: Option<GameMode>,
}

/// Game mode for player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

/// Player entity wrapper.
#[derive(Debug, Clone)]
pub struct PlayerEntity {
    pub entity: Entity,
    pub data: PlayerData,
}

impl PlayerEntity {
    pub fn new(id: ResourceId) -> Self {
        Self {
            entity: Entity::new(id, EntityType::Player),
            data: PlayerData {
                inventory: super::inventory::Inventory::new(
                    super::inventory::ContainerDefinition::player_inventory(),
                ),
                experience_level: 0,
                experience_points: 0.0,
                total_experience: 0,
                armor_items: [None, None, None, None],
                main_hand: None,
                off_hand: None,
                game_mode: GameMode::Survival,
                flying: false,
                creative: false,
                game_mode_previous: None,
            },
        }
    }
}

/// AI Goal - a behavior that an entity can pursue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiGoal {
    Idle,
    Wander,
    FollowTarget { target_id: ResourceId },
    AttackTarget { target_id: ResourceId },
    FleeTarget { target_id: ResourceId },
    Sleep,
    Swim,
    Fly,
}

/// AI controller for an entity.
#[derive(Debug, Clone)]
pub struct AIController {
    pub current_goal: AiGoal,
    pub tick_delay: u32,
    pub reaction_delay: u32,
    pub sight_range: f32,
    pub attack_range: f32,
    pub follow_range: f32,
    pub aggression: f32,
    pub fear_level: f32,
}

impl AIController {
    pub fn new(entity_type: EntityType) -> Self {
        match entity_type {
            EntityType::Zombie | EntityType::ZombieVillager => Self {
                current_goal: AiGoal::Idle,
                tick_delay: 20,
                reaction_delay: 0,
                sight_range: 35.0,
                attack_range: 2.0,
                follow_range: 35.0,
                aggression: 1.0,
                fear_level: 0.0,
            },
            EntityType::Skeleton => Self {
                current_goal: AiGoal::Idle,
                tick_delay: 20,
                reaction_delay: 0,
                sight_range: 16.0,
                attack_range: 8.0,
                follow_range: 16.0,
                aggression: 1.0,
                fear_level: 0.3,
            },
            EntityType::Creeper => Self {
                current_goal: AiGoal::Idle,
                tick_delay: 20,
                reaction_delay: 0,
                sight_range: 16.0,
                attack_range: 2.0,
                follow_range: 16.0,
                aggression: 1.0,
                fear_level: 0.2,
            },
            EntityType::Spider => Self {
                current_goal: AiGoal::Idle,
                tick_delay: 20,
                reaction_delay: 0,
                sight_range: 16.0,
                attack_range: 2.0,
                follow_range: 16.0,
                aggression: 0.5,
                fear_level: 0.2,
            },
            _ => Self {
                current_goal: AiGoal::Idle,
                tick_delay: 20,
                reaction_delay: 0,
                sight_range: 16.0,
                attack_range: 2.0,
                follow_range: 16.0,
                aggression: 0.0,
                fear_level: 0.0,
            },
        }
    }

    pub fn decide_action(&self, entity: &Entity, visible_entities: &[(ResourceId, Vec3)]) -> AiGoal {
        let nearest_hostile = visible_entities
            .iter()
            .filter(|(_, pos)| {
                let dist = entity.position.distance(*pos);
                dist < self.sight_range
            })
            .min_by(|(_, a), (_, b)| {
                let da = (entity.position - *a).length_squared();
                let db = (entity.position - *b).length_squared();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });

        match entity.ai_type {
            AIType::Passive => AiGoal::Wander,
            AIType::Neutral => {
                if let Some((target_id, _)) = nearest_hostile {
                    AiGoal::AttackTarget { target_id: target_id.clone() }
                } else {
                    AiGoal::Idle
                }
            }
            AIType::Hostile => {
                if let Some((target_id, _)) = nearest_hostile {
                    AiGoal::AttackTarget { target_id: target_id.clone() }
                } else {
                    AiGoal::Wander
                }
            }
            AIType::Water => AiGoal::Swim,
            AIType::Flying => AiGoal::Fly,
            AIType::PassivePeaceful => AiGoal::Wander,
        }
    }
}

/// Physics engine for entities.
pub struct PhysicsEngine {
    gravity: f32,
    terminal_velocity: f32,
    drag: f32,
}

impl PhysicsEngine {
    pub fn new() -> Self {
        Self {
            gravity: 0.08,
            terminal_velocity: 3.92,
            drag: 0.91,
        }
    }

    pub fn apply_physics(
        &mut self,
        entity: &mut Entity,
        dt: f32,
        world: &dyn WorldAccess,
    ) {
        if entity.on_ground {
            entity.velocity.y = 0.0;
        } else {
            entity.velocity.y -= self.gravity;
            if entity.velocity.y < -self.terminal_velocity {
                entity.velocity.y = -self.terminal_velocity;
            }
        }

        entity.velocity *= self.drag.powf(dt * 10.0);

        let delta = entity.velocity * dt;
        let mut new_pos = entity.position + delta;

        let box_size = Self::entity_type_collision_box(entity.entity_type);

        if world.is_position_colliding(entity.position, new_pos, box_size) {
            let mut tentative = entity.position;
            tentative.x += delta.x;
            if !world.is_position_colliding(entity.position, tentative, box_size) {
                new_pos.x = tentative.x;
                entity.velocity.x = 0.0;
            } else {
                entity.velocity.x = 0.0;
            }

            tentative = entity.position;
            tentative.y += delta.y;
            if !world.is_position_colliding(entity.position, tentative, box_size) {
                new_pos.y = tentative.y;
                if delta.y < 0.0 {
                    entity.on_ground = true;
                }
                entity.velocity.y = 0.0;
            } else {
                entity.velocity.y = 0.0;
            }

            tentative = entity.position;
            tentative.z += delta.z;
            if !world.is_position_colliding(entity.position, tentative, box_size) {
                new_pos.z = tentative.z;
                entity.velocity.z = 0.0;
            } else {
                entity.velocity.z = 0.0;
            }

            entity.position = new_pos;
        } else {
            entity.position = new_pos;
        }

        entity.on_ground = world.is_position_on_ground(entity.position);
    }

    pub fn entity_type_collision_box(entity_type: EntityType) -> (Vec3, Vec3) {
        match entity_type {
            EntityType::Player => (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.8, 0.3)),
            EntityType::Zombie | EntityType::ZombieVillager => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.99, 0.3))
            }
            EntityType::Skeleton => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.99, 0.3))
            }
            EntityType::Creeper => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.7, 0.3))
            }
            EntityType::Spider => {
                (Vec3::new(-0.7, 0.0, -0.7), Vec3::new(0.7, 0.9, 0.7))
            }
            EntityType::Pig => {
                (Vec3::new(-0.4, 0.0, -0.4), Vec3::new(0.4, 0.9, 0.4))
            }
            EntityType::Cow => {
                (Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.3, 0.5))
            }
            EntityType::Sheep => {
                (Vec3::new(-0.4, 0.0, -0.4), Vec3::new(0.4, 0.9, 0.4))
            }
            EntityType::Chicken => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.5, 0.3))
            }
            EntityType::Rabbit => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.4, 0.3))
            }
            EntityType::Wolf => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.85, 0.3))
            }
            EntityType::Ocelot => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.4, 0.3))
            }
            EntityType::FoxVariant => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.85, 0.3))
            }
            EntityType::Enderman => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 2.9, 0.3))
            }
            EntityType::Blaze => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.8, 0.3))
            }
            EntityType::Ghast => {
                (Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 1.0, 1.0))
            }
            EntityType::Witch => {
                (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 1.9, 0.3))
            }
            EntityType::Slime => {
                (Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 0.5, 0.5))
            }
            _ => (Vec3::new(-0.3, 0.0, -0.3), Vec3::new(0.3, 0.6, 0.3)),
        }
    }
}

/// World access interface for physics collision checks.
pub trait WorldAccess {
    fn is_position_colliding(
        &self,
        from: Vec3,
        to: Vec3,
        box_size: (Vec3, Vec3),
    ) -> bool;
    fn is_position_on_ground(&self, position: Vec3) -> bool;
    fn get_block(&self, x: i32, y: i32, z: i32) -> Option<ResourceId>;
}

/// Entity manager - tracks all entities in the world.
pub struct MobManager {
    pub entities: HashMap<ResourceId, Entity>,
    next_id: u32,
}

impl MobManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn spawn_entity(&mut self, entity_type: EntityType, position: Vec3) -> ResourceId {
        let id = ResourceId::minecraft(format!("entity_{}", self.next_id)).unwrap();
        let mut entity = Entity::new(id.clone(), entity_type);
        entity.position = position;
        self.next_id += 1;
        self.entities.insert(id.clone(), entity);
        id
    }

    pub fn remove_entity(&mut self, id: &ResourceId) {
        self.entities.remove(id);
    }

    pub fn get_entity(&self, id: &ResourceId) -> Option<&Entity> {
        self.entities.get(id)
    }

    pub fn get_entity_mut(&mut self, id: &ResourceId) -> Option<&mut Entity> {
        self.entities.get_mut(id)
    }

    pub fn tick(&mut self, world: &dyn WorldAccess) {
        let mut physics = PhysicsEngine::new();
        for entity in self.entities.values_mut() {
            if entity.is_alive() {
                physics.apply_physics(entity, 0.05, world);
                entity.ticks_existed += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_creation() {
        let entity = Entity::new(
            ResourceId::minecraft("test_zombie").unwrap(),
            EntityType::Zombie,
        );
        assert_eq!(entity.entity_type, EntityType::Zombie);
        assert_eq!(entity.max_health, 20.0);
        assert_eq!(entity.health, 20.0);
        assert_eq!(entity.attack_damage, 3.0);
        assert_eq!(entity.ai_type, AIType::Hostile);
    }

    #[test]
    fn entity_take_damage() {
        let mut entity = Entity::new(
            ResourceId::minecraft("test_zombie").unwrap(),
            EntityType::Zombie,
        );
        entity.take_damage(5.0);
        assert_eq!(entity.health, 15.0);
        assert!(entity.is_alive());

        entity.take_damage(20.0);
        assert_eq!(entity.health, 0.0);
        assert!(!entity.is_alive());
        assert_eq!(entity.status, EntityStatus::Dead);
    }

    #[test]
    fn entity_heal() {
        let mut entity = Entity::new(
            ResourceId::minecraft("test_zombie").unwrap(),
            EntityType::Zombie,
        );
        entity.take_damage(5.0);
        entity.heal(3.0);
        assert_eq!(entity.health, 18.0);
    }

    #[test]
    fn mob_manager_spawn() {
        let mut manager = MobManager::new();
        let id = manager.spawn_entity(EntityType::Zombie, Vec3::new(0.0, 64.0, 0.0));
        assert!(manager.entities.contains_key(&id));

        let entity = manager.get_entity(&id).unwrap();
        assert_eq!(entity.entity_type, EntityType::Zombie);
        assert_eq!(entity.position, Vec3::new(0.0, 64.0, 0.0));
    }

    #[test]
    fn mob_manager_remove() {
        let mut manager = MobManager::new();
        let id = manager.spawn_entity(EntityType::Zombie, Vec3::ZERO);
        assert!(manager.get_entity(&id).is_some());

        manager.remove_entity(&id);
        assert!(manager.get_entity(&id).is_none());
    }

    #[test]
    fn collision_box_player() {
        let (min, max) = PhysicsEngine::entity_type_collision_box(EntityType::Player);
        assert!((min.x - (-0.3)).abs() < f32::EPSILON);
        assert!((max.y - 1.8).abs() < f32::EPSILON);
    }
}