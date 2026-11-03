// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.

// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.#![forbid(unsafe_code)]

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

mod ui;

use clap::Parser;
use gibrid_core::{
    accounts::AccountManager,
    entity::{EntityWorld, Physics},
    AppConfig, Lifecycle, ResourceId, VersionManifest,
};
use gibrid_core::entity::EntityKind;
use gibrid_gameplay::{crafting::CraftingManager, input::InputHandler, WorldContext};
use gibrid_mesh::{mesh_queue::MeshQueue, FaceTextureMap};
use gibrid_renderer_wgpu::{Renderer, GfxBackend, ui_renderer::UiRenderer};
use gibrid_storage::{chunk::parse_chunk_sections, region::{index_for_chunk, read_chunk, RegionHeader}};
use ui::UiState;
use tracing::{error, info, warn};
use std::collections::HashMap;
use std::path::Path;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::Window;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("gibrid: {err}");
            ExitCode::from(1)
        }
    }
}

/// Аргументы командной строки ядра.
#[derive(Parser, Debug)]
#[command(author, version, about = "GIBRID Native Client")]
struct Args {
    /// Имя игрока (используется в заголовке окна и сессии).
    #[arg(long, default_value = "Steve")]
    username: String,
    /// Количество потоков рендеринга (резерв, wgpu управляет потоками сам).
    #[arg(long, default_value_t = 8)]
    render_threads: usize,
    /// Графический бэкенд: vulkan, dx12, gl, primary.
    #[arg(long, default_value = "vulkan")]
    gfx_backend: String,
    /// Ширина окна.
    #[arg(long, default_value_t = 800)]
    width: u32,
    /// Высота окна.
    #[arg(long, default_value_t = 600)]
    height: u32,
    /// Авто-подкачка ассетов из установленного Minecraft перед запуском.
    #[arg(long)]
    fetch_assets: bool,
    /// Путь к папке .minecraft (по умолчанию %APPDATA%\.minecraft).
    #[arg(long)]
    minecraft_dir: Option<PathBuf>,
    /// Путь к текстуре атласа от лаунчера (raw rgba, width height).
    #[arg(long)]
    atlas_data: Option<PathBuf>,
    /// Ширина атласа.
    #[arg(long, default_value_t = 0)]
    atlas_width: u32,
    /// Высота атласа.
    #[arg(long, default_value_t = 0)]
    atlas_height: u32,
    /// Директория с текстурами для сборки атласа (по умолчанию gibrid_assets/assets/minecraft/textures/block).
    #[arg(long)]
    atlas_dir: Option<PathBuf>,
    /// Размер плитки текстуры в атласе (по умолчанию 16).
    #[arg(long, default_value_t = 16)]
    atlas_tile_size: u32,
    /// Директория с ассетами лаунчера (где лежат indexes/ и objects/). Если указано — текстуры загружаются оттуда.
    #[arg(long)]
    assets_dir: Option<PathBuf>,
    /// Директория с извлечённым minecraft.jar (где лежит assets/minecraft/textures/block/). Если указано — текстуры загружаются оттуда.
    #[arg(long)]
    minecraft_assets: Option<PathBuf>,
    /// Папка мира (содержит region/). Если указано — загружается чанк.
    #[arg(long)]
    world_dir: Option<PathBuf>,
    /// Координата X чанка для загрузки.
    #[arg(long, default_value_t = 0)]
    chunk_x: i32,
    /// Координата Z чанка для загрузки.
    #[arg(long, default_value_t = 0)]
    chunk_z: i32,
}

/// Экран приветствия с выбором оффлайн-аккаунта (консольный, перед запуском окна).
fn login_screen() -> Result<String, Box<dyn std::error::Error>> {
    let app_dir = app_data_dir()?;
    let accounts_path = app_dir.join("accounts.toml");
    let mut mgr = AccountManager::load(&accounts_path)?;

    println!("=== GIBRID Native — вход (оффлайн) ===");
    println!("Аккаунты:");

    let accounts: Vec<String> = mgr.accounts().iter().map(|a| a.username.clone()).collect();
    if accounts.is_empty() {
        println!("  (аккаунтов нет, будет создан новый)");
    } else {
        for (i, name) in accounts.iter().enumerate() {
            let marker = if mgr.current_username() == Some(name) { " (текущий)" } else { "" };
            println!("  {}. {}{}", i + 1, name, marker);
        }
    }

    println!();
    println!("Команды:");
    println!("  <номер>   — войти под существующим аккаунтом");
    println!("  <имя>     — создать новый аккаунт и войти");
    println!("  enter     — войти под текущим (если есть) или создать \"Steve\"");
    print!("\n> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let username = if input.is_empty() {
        if let Some(cur) = mgr.current_username() {
            cur.to_string()
        } else {
            "Steve".to_string()
        }
    } else if let Ok(idx) = input.parse::<usize>() {
        if idx >= 1 && idx <= accounts.len() {
            accounts[idx - 1].clone()
        } else {
            input.to_string()
        }
    } else {
        input.to_string()
    };

    if mgr.get_account(&username).is_none() {
        mgr.add_account(&username)?;
        info!(subsystem = "accounts", event_id = "account_created", username = %username, "новый оффлайн-аккаунт");
    } else {
        let acc = mgr.get_account_mut(&username).unwrap();
        acc.touch_login();
    }

    mgr.set_current(&username)?;
    mgr.save(&accounts_path)?;

    info!(subsystem = "accounts", event_id = "account_selected", username = %username, "аккаунт выбран");
    Ok(username)
}

fn app_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(appdata) = env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("GIBRID");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    } else {
        let dir = env::current_dir()?.join("gibrid_data");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}

/// Загрузить чанк из региона файла (.mca) мира Minecraft в WorldContext.
fn load_chunk_from_region(
    world: &mut WorldContext,
    world_dir: &Path,
    chunk_x: i32,
    chunk_z: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let region_x = chunk_x.div_euclid(32);
    let region_z = chunk_z.div_euclid(32);
    let region_file = world_dir.join("region").join(format!("r.{}.{}.mca", region_x, region_z));

    if !region_file.exists() {
        warn!(subsystem = "world", event_id = "region_not_found", file = %region_file.display(), "файл региона не найден");
        return Ok(());
    }

    let region_bytes = std::fs::read(&region_file)?;
    let header = RegionHeader::parse(&region_bytes)?;
    let index = index_for_chunk(chunk_x, chunk_z);

    let loc = match header.location(index) {
        Some(loc) => loc,
        None => {
            warn!(subsystem = "world", event_id = "chunk_not_found", x = chunk_x, z = chunk_z, "чанк не найден в регионе");
            return Ok(());
        }
    };

    let raw_chunk = read_chunk(&region_bytes, loc)?;
    let nbt_bytes = raw_chunk.decompress()?;

    let sections = parse_chunk_sections(&nbt_bytes)?;

    let mut blocks_loaded = 0u32;
    for section in sections {
        let section_y = section.y as i32 * 16;
        let palette = &section.block_states.entries;

        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let idx = section.block_index_at(x, y, z).unwrap_or(0);
                    if let Some(entry) = palette.get(idx) {
                        if entry.name.path() == "air" {
                            continue;
                        }
                        let global_x = chunk_x * 16 + x as i32;
                        let global_y = section_y + y as i32;
                        let global_z = chunk_z * 16 + z as i32;
                        world.set_block(global_x, global_y, global_z, Some(entry.name.clone()));
                        blocks_loaded += 1;
                    }
                }
            }
        }
    }

    info!(
        subsystem = "world",
        event_id = "chunk_loaded",
        x = chunk_x,
        z = chunk_z,
        blocks = blocks_loaded,
        "чанк загружен из региона"
    );
    Ok(())
}

/// Построить текстурный атлас: из файла (лаунчер), из ассетами реального майна, или из gibrid_assets/.
/// Возвращает (ширина, высота, данные_rgba, карта_текстур).
fn build_texture_atlas(
    args: &Args,
) -> Result<(u32, u32, Vec<u8>, HashMap<ResourceId, u32>), Box<dyn std::error::Error>> {
    // Если передан готовый файл атласа — используем его
    if let Some(atlas_path) = &args.atlas_data {
        let data = std::fs::read(atlas_path)?;
        return Ok((args.atlas_width, args.atlas_height, data, HashMap::new()));
    }

    // Если указан assets_dir (реальные ассеты лаунчера через индекс) — загружаем оттуда
    if let Some(assets_dir) = &args.assets_dir {
        let atlas = gibrid_assets::texture::load_atlas_from_asset_index(assets_dir, args.atlas_tile_size)?;
        info!(
            subsystem = "renderer",
            event_id = "atlas_loaded_from_assets",
            width = atlas.width,
            height = atlas.height,
            textures = atlas.regions.len(),
            assets_dir = %assets_dir.display(),
            "текстурный атлас загружен из реальных ассетов майна"
        );
        if atlas.width == 0 || atlas.height == 0 {
            let white: [u8; 4] = [255, 255, 255, 255];
            return Ok((1, 1, white.to_vec(), HashMap::new()));
        }
        let texture_map: HashMap<ResourceId, u32> = atlas
            .regions
            .keys()
            .enumerate()
            .map(|(i, id_str)| {
                let id_str = id_str.strip_prefix("minecraft:").unwrap_or(id_str);
                let id = ResourceId::minecraft(id_str).unwrap_or_else(|_| ResourceId::minecraft("missing").unwrap());
                (id, i as u32)
            })
            .collect();
        return Ok((atlas.width, atlas.height, atlas.raw_rgba, texture_map));
    }

    // Если указан minecraft_assets (извлечённый minecraft.jar) — загружаем текстуры блоков напрямую
    if let Some(mc_assets) = &args.minecraft_assets {
        let atlas = gibrid_assets::texture::load_and_build_atlas(mc_assets, args.atlas_tile_size)?;
        info!(
            subsystem = "renderer",
            event_id = "atlas_loaded_from_minecraft_assets",
            width = atlas.width,
            height = atlas.height,
            textures = atlas.regions.len(),
            minecraft_assets = %mc_assets.display(),
            "текстурный атлас загружен из извлечённого minecraft.jar"
        );
        if atlas.width == 0 || atlas.height == 0 {
            let white: [u8; 4] = [255, 255, 255, 255];
            return Ok((1, 1, white.to_vec(), HashMap::new()));
        }
        let texture_map: HashMap<ResourceId, u32> = atlas
            .regions
            .keys()
            .enumerate()
            .map(|(i, id_str)| {
                let id_str = id_str.strip_prefix("minecraft:").unwrap_or(id_str);
                let id = ResourceId::minecraft(id_str).unwrap_or_else(|_| ResourceId::minecraft("missing").unwrap());
                (id, i as u32)
            })
            .collect();
        return Ok((atlas.width, atlas.height, atlas.raw_rgba, texture_map));
    }

    // Иначе собираем атлас из gibrid_assets/
    let assets_dir: PathBuf = args.atlas_dir.clone().unwrap_or_else(|| PathBuf::from("gibrid_assets"));
    let atlas = gibrid_assets::texture::load_and_build_atlas(&assets_dir, args.atlas_tile_size)?;
    info!(
        subsystem = "renderer",
        event_id = "atlas_built",
        width = atlas.width,
        height = atlas.height,
        textures = atlas.regions.len(),
        "текстурный атлас собран из ассетов"
    );

    if atlas.width == 0 || atlas.height == 0 {
        warn!(subsystem = "renderer", event_id = "atlas_empty", "текстур не найдено, используется заглушка 1x1");
        let white: [u8; 4] = [255, 255, 255, 255];
        Ok((1, 1, white.to_vec(), HashMap::new()))
    } else {
        let texture_map: HashMap<ResourceId, u32> = atlas
            .regions
            .keys()
            .enumerate()
            .map(|(i, id_str)| {
                let id_str = id_str.strip_prefix("minecraft:").unwrap_or(id_str);
                let id = ResourceId::minecraft(id_str).unwrap_or_else(|_| ResourceId::minecraft("missing").unwrap());
                (id, i as u32)
            })
            .collect();
        Ok((atlas.width, atlas.height, atlas.raw_rgba, texture_map))
    }
}

#[allow(deprecated)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Автоподкачка ассетов из основного майна
    if args.fetch_assets {
        let mc_dir = match &args.minecraft_dir {
            Some(p) => p.clone(),
            None => default_minecraft_dir(),
        };
        gibrid_assets::fetch_from_minecraft(&mc_dir)?;
    }

    let username = login_screen()?;

    let config = load_config()?;
    tracing_subscriber::fmt()
        .with_env_filter(config.log_level.clone())
        .with_target(false)
        .init();

    let mut life = Lifecycle::Created;
    life = life
        .transition(Lifecycle::Initializing)
        .expect("created -> init");

    let (manifest_path, manifest) = load_manifest()?;
    init_logging_fields(&config, &manifest_path, &manifest);

    info!(
        subsystem = "app",
        event_id = "startup",
        username = %username,
        render_threads = args.render_threads,
        gfx_backend = %args.gfx_backend,
        "gibrid native phase 5 gui"
    );

    life = life
        .transition(Lifecycle::Ready)
        .and_then(|s| s.transition(Lifecycle::Running))
        .map_err(|_| "illegal lifecycle transition")?;

    let backend = match args.gfx_backend.to_lowercase().as_str() {
        "vulkan" => GfxBackend::Vulkan,
        "dx12" => GfxBackend::Dx12,
        "gl" => GfxBackend::Gl,
        _ => GfxBackend::Primary,
    };

    let event_loop = EventLoop::new().map_err(|e| format!("failed to create event loop: {e}"))?;
    let window_attributes = Window::default_attributes()
        .with_title(format!("GIBRID Native — {} (оффлайн)", username))
        .with_inner_size(LogicalSize::new(args.width, args.height));
    let window = event_loop
        .create_window(window_attributes)
        .map_err(|e| format!("failed to create window: {e}"))?;

    let app_window_id = window.id();

    let mut renderer = Renderer::new(window, backend, args.render_threads)?;

    // Текстурный атлас: из файла (лаунчер) или из gibrid_assets/
    let (atlas_width, atlas_height, atlas_data, texture_map) = build_texture_atlas(&args)?;
    renderer.set_texture_atlas(atlas_width, atlas_height, &atlas_data);
    info!(
        subsystem = "renderer",
        event_id = "atlas_uploaded",
        width = atlas_width,
        height = atlas_height,
        bytes = atlas_data.len(),
        textures = texture_map.len(),
        "атлас загружен на GPU"
    );

    // Карта текстур для использования в мире (передаётся в замыкание события)
    let texture_map_arc = std::sync::Arc::new(texture_map);
    let face_texture_map_arc = std::sync::Arc::new(HashMap::new());
    let atlas_width = atlas_width;
    let atlas_height = atlas_height;
    let tile_size = args.atlas_tile_size;

    // UI-рендерер
    let mut ui_renderer = UiRenderer::new(renderer.device(), renderer.queue(), renderer.config().format);

    // Состояние меню
    let mut ui_state = UiState::main_menu();
    let mut in_game = false;
    let mut inventory_open = false;
    let mut crafting_manager = CraftingManager::default();

    // Состояние крафта
    let mut craft_grid: [Option<ResourceId>; 9] = [const { None }; 9];
    let mut craft_result: Option<(ResourceId, u32)> = None;

    // Мир и сущности (создаются при входе в игру)
    let mut world: Option<WorldContext> = None;
    let mut entities: Option<EntityWorld> = None;
    let mut player_id: Option<u64> = None;
    let mut player = gibrid_gameplay::Player::new(0.0, 64.0, 0.0);
    let mut input = InputHandler::new();

    info!(subsystem = "gameplay", event_id = "ui_ready", "главное меню готово");

    let mut last_instant = Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app_window_id => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state,
                            physical_key: PhysicalKey::Code(code),
                            ..
                        },
                    ..
                } => {
                    if *state == ElementState::Pressed {
                        match code {
                            winit::keyboard::KeyCode::Escape => {
                                if inventory_open {
                                    inventory_open = false;
                                    info!(subsystem = "app", event_id = "inventory_closed", "инвентарь закрыт");
                                } else if in_game {
                                    // Выход в главное меню
                                    in_game = false;
                                    ui_state = UiState::main_menu();
                                    info!(subsystem = "app", event_id = "menu_opened", "главное меню");
                                } else {
                                    elwt.exit();
                                }
                            }
                            winit::keyboard::KeyCode::KeyE => {
                                if in_game {
                                    inventory_open = !inventory_open;
                                    if inventory_open {
                                        info!(subsystem = "app", event_id = "inventory_opened", "инвентарь открыт");
                                    } else {
                                        info!(subsystem = "app", event_id = "inventory_closed", "инвентарь закрыт");
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    // Передаём ввод игроку только если в игре и инвентарь закрыт
                    if in_game && !inventory_open {
                        if let Some(ref mut w) = world {
                            input.handle_event(event, &mut player);
                        }
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(*physical_size);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let mx = position.x as f32 / renderer.size().width as f32;
                    let my = position.y as f32 / renderer.size().height as f32;
                    let clicked = ui_state.update_mouse(mx, my, false);
                    if let Some(btn_id) = clicked {
                        handle_ui_click(btn_id, &mut ui_state, &mut in_game, &mut world, &mut entities, &mut player_id, &mut inventory_open, &args, &mut crafting_manager, &mut craft_grid, &mut craft_result, &mut player, &texture_map_arc, &face_texture_map_arc, atlas_width, atlas_height, tile_size);
                    }
                }
                WindowEvent::MouseInput { button, state: ElementState::Pressed, .. } => {
                    if !in_game {
                        let clicked = ui_state.update_mouse(ui_state.mouse_x, ui_state.mouse_y, true);
                        if let Some(btn_id) = clicked {
                            handle_ui_click(btn_id, &mut ui_state, &mut in_game, &mut world, &mut entities, &mut player_id, &mut inventory_open, &args, &mut crafting_manager, &mut craft_grid, &mut craft_result, &mut player, &texture_map_arc, &face_texture_map_arc, atlas_width, atlas_height, tile_size);
                        }
                    } else if !inventory_open {
                        // В игре: ЛКМ ломает блок, ПКМ ставит (только если инвентарь закрыт)
                        if let Some(ref mut w) = world {
                            match button {
                                MouseButton::Left => {
                                    let stone = ResourceId::minecraft("stone").unwrap();
                                    w.set_block(0, 63, 0, None);
                                    // Добавляем камень в инвентарь игрока
                                    let remaining = player.inventory.add_item(stone.clone(), 1);
                                    info!(
                                        subsystem = "gameplay",
                                        event_id = "block_broken",
                                        x = 0, y = 63, z = 0,
                                        added = 1 - remaining,
                                        "блок сломан, предмет в инвентарь"
                                    );
                                }
                                MouseButton::Right => {
                                    let stone = ResourceId::minecraft("stone").unwrap();
                                    w.set_block(3, 63, 0, Some(stone.clone()));
                                    info!(subsystem = "gameplay", event_id = "block_placed", x = 3, y = 63, z = 0);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                let now = Instant::now();
                let dt = now.duration_since(last_instant).as_secs_f32();
                last_instant = now;
                let dt = dt.clamp(0.0, 0.1);

                if in_game {
                    // Движение игрока через InputHandler
                    if let Some(ref mut w) = world {
                        if let Some(pid) = player_id {
                            let move_vec = input.movement_vector(&player);
                            if move_vec.length() > 0.0 {
                                player.position += move_vec.normalize() * 5.0 * dt;
                            }

                            // Обновляем сущность игрока
                            if let Some(e) = entities.as_mut().unwrap().get_mut(pid) {
                                e.position = player.position;
                                e.yaw = player.yaw;
                                e.pitch = player.pitch;
                                Physics::apply_gravity(e, dt);
                                Physics::integrate(e, dt);
                            }

                            // Загрузка мешей из очереди
                            while let Some(result) = w.try_recv_mesh() {
                                renderer.upload_chunk(&result);
                                info!(subsystem = "renderer", event_id = "chunk_uploaded", section_x = result.mesh.section_x, section_y = result.mesh.section_y, section_z = result.mesh.section_z, index_count = result.mesh.index_count, "чанк загружен на GPU");
                            }
                        }
                    }
                }

                // Рендер кадра: мир, инвентарь или UI-меню
                if in_game {
                    // Обновить камеру из позиции игрока
                    if let Some(pid) = player_id {
                        let pos = player.position;
                        renderer.update_camera([pos.x, pos.y, pos.z], player.yaw, player.pitch);
                    }

                    if let Err(err) = renderer.render() {
                        match err {
                            wgpu::SurfaceError::Lost => {
                                let size = renderer.size();
                                renderer.resize(size);
                            }
                            wgpu::SurfaceError::OutOfMemory => elwt.exit(),
                            _ => {}
                        }
                    }

                    // Отрисовать инвентарь поверх мира, если открыт
                    if inventory_open {
                        let inv = &player.inventory;
                        let inv_slot_size = 0.04;
                        let inv_start_x = 0.20;
                        let inv_start_y = 0.42;
                        let inv_cols = 9;
                        let inv_rows = 4;

                        // Крафт сетка 3x3
                        let craft_slot_size = 0.045;
                        let craft_start_x = 0.20;
                        let craft_start_y = 0.25;
                        let craft_cols = 3;
                        let craft_rows = 3;

                        let mut all_buttons: Vec<gibrid_renderer_wgpu::ui_renderer::UiButton> = Vec::new();

                        // Крафт слоты
                        for row in 0..craft_rows {
                            for col in 0..craft_cols {
                                let idx = row * craft_cols + col;
                                let x = craft_start_x + col as f32 * (craft_slot_size + 0.012);
                                let y = craft_start_y + row as f32 * (craft_slot_size + 0.012);
                                let color = if let Some(item_id) = &craft_grid[idx] {
                                    match item_id.path() {
                                        "stone" => [0.5, 0.5, 0.5, 1.0],
                                        "cobblestone" => [0.4, 0.4, 0.4, 1.0],
                                        "dirt" => [0.4, 0.3, 0.2, 1.0],
                                        _ => [0.8, 0.8, 0.8, 1.0],
                                    }
                                } else {
                                    [0.15, 0.15, 0.15, 1.0]
                                };
                                all_buttons.push(gibrid_renderer_wgpu::ui_renderer::UiButton {
                                    id: format!("craft_{}", idx),
                                    label: String::new(),
                                    x,
                                    y,
                                    w: craft_slot_size,
                                    h: craft_slot_size,
                                    color,
                                    hover_color: [color[0], color[1], color[2], 1.0],
                                });
                            }
                        }

                        // Слот результата крафта
                        let result_x = craft_start_x + (craft_cols as f32 + 1.0) * (craft_slot_size + 0.012);
                        let result_y = craft_start_y + 1.0 * (craft_slot_size + 0.012);
                        let result_color = if let Some((item_id, _)) = &craft_result {
                            match item_id.path() {
                                "stone" => [0.5, 0.5, 0.5, 1.0],
                                "cobblestone" => [0.4, 0.4, 0.4, 1.0],
                                _ => [0.8, 0.8, 0.8, 1.0],
                            }
                        } else {
                            [0.15, 0.15, 0.15, 1.0]
                        };
                        all_buttons.push(gibrid_renderer_wgpu::ui_renderer::UiButton {
                            id: "craft_result".to_string(),
                            label: String::new(),
                            x: result_x,
                            y: result_y,
                            w: craft_slot_size,
                            h: craft_slot_size,
                            color: result_color,
                            hover_color: [result_color[0], result_color[1], result_color[2], 1.0],
                        });

                        // Кнопка крафт
                        let craft_btn_x = craft_start_x;
                        let craft_btn_y = craft_start_y + (craft_rows as f32 + 0.5) * (craft_slot_size + 0.012);
                        all_buttons.push(gibrid_renderer_wgpu::ui_renderer::UiButton {
                            id: "craft_button".to_string(),
                            label: "Крафт".to_string(),
                            x: craft_btn_x,
                            y: craft_btn_y,
                            w: 0.08,
                            h: 0.05,
                            color: [0.3, 0.6, 0.3, 1.0],
                            hover_color: [0.4, 0.7, 0.4, 1.0],
                        });

                        // Инвентарь слоты (ниже крафта)
                        for row in 0..inv_rows {
                            for col in 0..inv_cols {
                                let idx = row * inv_cols + col;
                                let x = inv_start_x + col as f32 * (inv_slot_size + 0.01);
                                let y = inv_start_y + row as f32 * (inv_slot_size + 0.01);
                                let color = if let Some(stack) = inv.slots.get(idx).and_then(|s| s.as_ref()) {
                                    match stack.item_id.path() {
                                        "stone" => [0.5, 0.5, 0.5, 1.0],
                                        "cobblestone" => [0.4, 0.4, 0.4, 1.0],
                                        "dirt" => [0.4, 0.3, 0.2, 1.0],
                                        _ => [0.8, 0.8, 0.8, 1.0],
                                    }
                                } else {
                                    [0.15, 0.15, 0.15, 1.0]
                                };
                                all_buttons.push(gibrid_renderer_wgpu::ui_renderer::UiButton {
                                    id: format!("slot_{}", idx),
                                    label: String::new(),
                                    x,
                                    y,
                                    w: inv_slot_size,
                                    h: inv_slot_size,
                                    color,
                                    hover_color: [color[0], color[1], color[2], 1.0],
                                });
                            }
                        }

                        if let Err(err) = renderer.render_ui(&mut ui_renderer, &all_buttons) {
                            match err {
                                wgpu::SurfaceError::Lost => {
                                    let size = renderer.size();
                                    renderer.resize(size);
                                }
                                wgpu::SurfaceError::OutOfMemory => elwt.exit(),
                                _ => {}
                            }
                        }
                    }
                } else {
                    let buttons: Vec<gibrid_renderer_wgpu::ui_renderer::UiButton> = ui_state
                        .buttons
                        .iter()
                        .map(|b| gibrid_renderer_wgpu::ui_renderer::UiButton {
                            id: b.id.to_string(),
                            label: b.label.to_string(),
                            x: b.rect.x,
                            y: b.rect.y,
                            w: b.rect.w,
                            h: b.rect.h,
                            color: [0.2, 0.4, 0.6, 1.0],
                            hover_color: [0.4, 0.6, 0.8, 1.0],
                        })
                        .collect();
                    if let Err(err) = renderer.render_ui(&mut ui_renderer, &buttons) {
                        match err {
                            wgpu::SurfaceError::Lost => {
                                let size = renderer.size();
                                renderer.resize(size);
                            }
                            wgpu::SurfaceError::OutOfMemory => elwt.exit(),
                            _ => {}
                        }
                    }
                }

                // ControlFlow::Poll вызывает AboutToWait continuamente — рендер идёт каждый кадр.
            }
            _ => {}
        }
    })?;

    life = life
        .transition(Lifecycle::ShuttingDown)
        .and_then(|s| s.transition(Lifecycle::Stopped))
        .map_err(|_| "illegal lifecycle transition")?;

    info!(
        subsystem = "app",
        event_id = "shutdown",
        lifecycle = ?life,
        "clean exit"
    );
    Ok(())
}

/// Обработать клик по кнопке меню.
fn handle_ui_click(
    btn_id: &str,
    ui_state: &mut UiState,
    in_game: &mut bool,
    world: &mut Option<WorldContext>,
    entities: &mut Option<EntityWorld>,
    player_id: &mut Option<u64>,
    inventory_open: &mut bool,
    args: &Args,
    crafting_manager: &mut CraftingManager,
    craft_grid: &mut [Option<ResourceId>; 9],
    craft_result: &mut Option<(ResourceId, u32)>,
    player: &mut gibrid_gameplay::Player,
    texture_map_arc: &std::sync::Arc<HashMap<ResourceId, u32>>,
    face_texture_map_arc: &std::sync::Arc<FaceTextureMap>,
    atlas_width: u32,
    atlas_height: u32,
    tile_size: u32,
) {
    match btn_id {
        "singleplayer" => {
            // Войти в игру: создать мир, игрока, сущность
            let mesh_queue = MeshQueue::new(16, 16);
            let texture_map = texture_map_arc.clone();
            *world = Some(WorldContext::new(mesh_queue, texture_map, face_texture_map_arc.clone(), atlas_width, atlas_height, tile_size));
            *entities = Some(EntityWorld::new());
            if let Some(ref mut e) = *entities {
                let pid = e.spawn(EntityKind::Player, glam::Vec3::new(0.0, 68.0, 0.0));
                *player_id = Some(pid);
            }
            // Смотрим вниз на блоки: pitch -15°
            player.yaw = 0.0;
            player.pitch = -15.0;
            player.position = glam::Vec3::new(0.0, 68.0, 0.0);
            // Тестовые блоки перед камерой (камера смотрит вдоль +X, pitch -15°)
            let stone = ResourceId::minecraft("stone").unwrap();
            if let Some(ref mut w) = *world {
                w.set_block(2, 68, 0, Some(stone.clone()));
                w.set_block(3, 68, 0, Some(stone.clone()));
                w.set_block(2, 67, 0, Some(stone.clone()));
                w.set_block(3, 67, 0, Some(stone.clone()));
                w.set_block(2, 68, 1, Some(stone.clone()));
                w.set_block(3, 68, 1, Some(stone.clone()));
            }
            *in_game = true;
            *inventory_open = false;
            *ui_state = UiState::singleplayer();
            info!(subsystem = "app", event_id = "menu_singleplayer", "вход в одиночную игру");
        }
        "multiplayer" => {
            *ui_state = UiState::multiplayer();
            info!(subsystem = "app", event_id = "menu_multiplayer", "экран мультиплеера");
        }
        "settings" => {
            *ui_state = UiState::settings();
            info!(subsystem = "app", event_id = "menu_settings", "экран настроек");
        }
        "quit" => {
            std::process::exit(0);
        }
        "back" => {
            *ui_state = UiState::main_menu();
            info!(subsystem = "app", event_id = "menu_back", "возврат в главное меню");
        }
        "add_server" => {
            info!(subsystem = "app", event_id = "menu_add_server", "добавить сервер (заглушка)");
        }
        // Крафт слот 0..8
        id if id.starts_with("craft_") && id != "craft_result" && id != "craft_button" => {
            let slot_idx: usize = id[6..].parse().unwrap_or(0);
            if slot_idx < 9 {
                // Перемещаем предмет между инвентарем и слотом крафта
                if let Some(item_id) = craft_grid[slot_idx].take() {
                    // Из крафта в инвентарь
                    let remaining = player.inventory.add_item(item_id.clone(), 1);
                    if remaining > 0 {
                        craft_grid[slot_idx] = Some(item_id);
                    }
                } else {
                    // Из инвентаря в слот крафта (первый найденный слот с предметом)
                    if let Some((inv_idx, stack)) = player
                        .inventory
                        .slots
                        .iter()
                        .enumerate()
                        .find_map(|(i, s)| s.as_ref().map(|st| (i, st.clone())))
                    {
                        craft_grid[slot_idx] = Some(stack.item_id.clone());
                        if stack.count > 1 {
                            player.inventory.slots[inv_idx] = Some(
                                gibrid_gameplay::inventory::ItemStack {
                                    item_id: stack.item_id.clone(),
                                    count: stack.count - 1,
                                    components: Default::default(),
                                },
                            );
                        } else {
                            player.inventory.slots[inv_idx] = None;
                        }
                    }
                }
            }
        }
        "craft_result" => {
            // Забрать результат крафта в инвентарь
            if let Some((item_id, count)) = craft_result.take() {
                let remaining = player.inventory.add_item(item_id.clone(), count);
                if remaining > 0 {
                    *craft_result = Some((item_id, remaining));
                }
            }
        }
        "craft_button" => {
            // Скрафтить предмет
            if let Some((recipe_idx, used_slots)) = crafting_manager.craft(&craft_grid) {
                if let Some(recipe) = crafting_manager.get_recipe(recipe_idx) {
                    // Забираем ингредиенты из сетки крафта
                    for slot_idx in used_slots {
                        craft_grid[slot_idx] = None;
                    }
                    // Добавляем результат в инвентарь
                    let result_item = recipe.result_item.clone();
                    let result_count = recipe.result_count;
                    let remaining = player.inventory.add_item(result_item.clone(), result_count);
                    if remaining > 0 {
                        *craft_result = Some((result_item, remaining));
                    } else {
                        *craft_result = None;
                    }
                }
            }
        }
        // Слот инвентаря slot_0..slot_35 — переместить предмет между инвентарем и крафтом
        id if id.starts_with("slot_") => {
            let slot_idx: usize = id[5..].parse().unwrap_or(0);
            if slot_idx < player.inventory.slots.len() {
                if let Some(stack) = player.inventory.slots[slot_idx].clone() {
                    let mut target_slot = None;
                    for (i, slot) in craft_grid.iter().enumerate() {
                        match slot {
                            None => {
                                target_slot = Some(i);
                                break;
                            }
                            Some(craft_id) if *craft_id == stack.item_id => {
                                target_slot = Some(i);
                                break;
                            }
                            _ => {}
                        }
                    }
                    if let Some(target) = target_slot {
                        craft_grid[target] = Some(stack.item_id.clone());
                        if stack.count > 1 {
                            player.inventory.slots[slot_idx] = Some(
                                gibrid_gameplay::inventory::ItemStack {
                                    item_id: stack.item_id.clone(),
                                    count: stack.count - 1,
                                    components: Default::default(),
                                },
                            );
                        } else {
                            player.inventory.slots[slot_idx] = None;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn default_minecraft_dir() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        PathBuf::from(appdata).join(".minecraft")
    } else {
        PathBuf::from(".minecraft")
    }
}

fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    if let Ok(path) = env::var("GIBRID_CONFIG") {
        let text = std::fs::read_to_string(path)?;
        return Ok(AppConfig::parse(&text)?);
    }
    Ok(AppConfig::default())
}

fn load_manifest() -> Result<(PathBuf, VersionManifest), Box<dyn std::error::Error>> {
    if let Ok(path) = env::var("GIBRID_VERSION_FILE") {
        let path = PathBuf::from(path);
        let manifest = VersionManifest::load_path(&path)?;
        return Ok((path, manifest));
    }
    let cwd = env::current_dir()?;
    Ok(VersionManifest::load_from_dir(cwd)?)
}

fn init_logging_fields(
    config: &AppConfig,
    manifest_path: &std::path::Path,
    manifest: &VersionManifest,
) {
    info!(
        subsystem = "app",
        event_id = "startup",
        crate_version = env!("CARGO_PKG_VERSION"),
        game_version = manifest.game_version.as_str(),
        protocol_version = manifest.protocol_version,
        protocol_verified = manifest.protocol_verified,
        data_version = ?manifest.data_version,
        confidence = manifest.confidence.as_str(),
        schema_version = config.schema_version,
        manifest = %manifest_path.display(),
        "gibrid native phase 5"
    );
    for warning in manifest.warnings() {
        warn!(
            subsystem = "version",
            event_id = "unverified_manifest",
            "{warning}"
        );
    }
    if let Err(err) = config.validate() {
        error!(subsystem = "config", event_id = "invalid_config", "{err}");
    }
}
