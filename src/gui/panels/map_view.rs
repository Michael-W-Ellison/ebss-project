// src/gui/panels/map_view.rs
//! Interactive world map rendering with camera controls.

use egui::{Ui, Sense, Color32, Rect, Pos2, Vec2, Stroke, Key};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::gui::state::{GuiState, SimulationCommand, EntitySelection, AgentSnapshot, AgentMapFilter};
use crate::world::TerrainType;
use crate::agents::LifeStage;
use super::tooltip;

const TILE_SIZE: f32 = 12.0;
const PAN_SPEED: f32 = 20.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 4.0;

pub fn render_map(
    ui: &mut Ui,
    state: &mut GuiState,
    command_tx: &Sender<SimulationCommand>,
    agent_data_request: &Arc<Mutex<Option<Uuid>>>,
) {
    if state.latest_snapshot.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("Waiting for simulation data...");
        });
        return;
    }

    let available_size = ui.available_size();

    // Handle follow mode - center on selected agent each frame
    // Must happen before we borrow snapshot
    if state.follow_selected {
        if let EntitySelection::Agent(id) = &state.selected {
            if let Some(snapshot) = &state.latest_snapshot {
                if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == *id && a.is_alive) {
                    let view_size = (available_size.x * 0.8, available_size.y * 0.8);
                    let world_x = agent.position.0 as f32 * TILE_SIZE * state.map_zoom;
                    let world_y = agent.position.1 as f32 * TILE_SIZE * state.map_zoom;
                    state.map_offset = (
                        world_x - view_size.0 / 2.0,
                        world_y - view_size.1 / 2.0,
                    );
                }
            }
        }
    }

    // Handle keyboard input for panning and zooming
    handle_keyboard_input(ui, state);

    // Now borrow snapshot for the rest of the function
    let snapshot = state.latest_snapshot.as_ref().unwrap();
    let world = &snapshot.world;

    // Main map area
    let map_width = world.width as f32 * TILE_SIZE * state.map_zoom;
    let map_height = world.height as f32 * TILE_SIZE * state.map_zoom;

    // Calculate visible area based on offset
    let (response, painter) = ui.allocate_painter(
        Vec2::new(available_size.x, available_size.y - 60.0), // Leave room for controls
        Sense::click_and_drag(),
    );

    let view_rect = response.rect;

    // Handle drag to pan
    if response.dragged() {
        let delta = response.drag_delta();
        state.map_offset.0 -= delta.x;
        state.map_offset.1 -= delta.y;
    }

    // Handle zoom with scroll wheel (zoom toward cursor)
    let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll_delta != 0.0 && response.hovered() {
        if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
            let old_zoom = state.map_zoom;
            let zoom_factor = 1.0 + scroll_delta * 0.002;
            state.map_zoom = (state.map_zoom * zoom_factor).clamp(MIN_ZOOM, MAX_ZOOM);

            // Adjust offset to zoom toward cursor position
            let cursor_rel = (cursor_pos.x - view_rect.min.x, cursor_pos.y - view_rect.min.y);
            let world_x = state.map_offset.0 + cursor_rel.0;
            let world_y = state.map_offset.1 + cursor_rel.1;
            let scale_change = state.map_zoom / old_zoom;
            state.map_offset.0 = world_x * scale_change - cursor_rel.0;
            state.map_offset.1 = world_y * scale_change - cursor_rel.1;
        }
    }

    // Clamp offset to keep map in view
    let max_offset_x = (map_width - view_rect.width()).max(0.0);
    let max_offset_y = (map_height - view_rect.height()).max(0.0);
    state.map_offset.0 = state.map_offset.0.clamp(-view_rect.width() * 0.5, max_offset_x + view_rect.width() * 0.5);
    state.map_offset.1 = state.map_offset.1.clamp(-view_rect.height() * 0.5, max_offset_y + view_rect.height() * 0.5);

    // Draw background
    painter.rect_filled(view_rect, 0.0, Color32::from_rgb(20, 20, 30));

    // Calculate visible tile range for culling
    let start_tile_x = ((state.map_offset.0 / (TILE_SIZE * state.map_zoom)).floor() as i32).max(0);
    let start_tile_y = ((state.map_offset.1 / (TILE_SIZE * state.map_zoom)).floor() as i32).max(0);
    let end_tile_x = (((state.map_offset.0 + view_rect.width()) / (TILE_SIZE * state.map_zoom)).ceil() as i32 + 1).min(world.width as i32);
    let end_tile_y = (((state.map_offset.1 + view_rect.height()) / (TILE_SIZE * state.map_zoom)).ceil() as i32 + 1).min(world.height as i32);

    // Draw terrain tiles (with culling)
    if state.map_layers.terrain {
        for tile in &world.tiles {
            if tile.x < start_tile_x || tile.x >= end_tile_x || tile.y < start_tile_y || tile.y >= end_tile_y {
                continue;
            }

            let screen_pos = world_to_screen(tile.x, tile.y, view_rect, state);
            let size = TILE_SIZE * state.map_zoom;

            let tile_rect = Rect::from_min_size(screen_pos, Vec2::new(size, size));
            if !view_rect.intersects(tile_rect) {
                continue;
            }

            let color = terrain_color(tile.terrain);
            painter.rect_filled(tile_rect, 0.0, color);
        }
    }

    // Draw grid overlay
    if state.map_layers.grid {
        draw_grid(&painter, view_rect, state, world.width, world.height);
    }

    // Draw resources
    if state.map_layers.resources {
        for resource in &world.resources {
            let screen_pos = world_to_screen(resource.position.x, resource.position.y, view_rect, state);
            let size = TILE_SIZE * state.map_zoom;

            if !view_rect.contains(screen_pos) {
                continue;
            }

            let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);
            let radius = size * 0.3;

            let color = resource_color(resource.resource_type);
            painter.circle_filled(center, radius, color);

            // Resource amount indicator (small arc)
            let fill_ratio = resource.amount as f32 / resource.max_amount.max(1) as f32;
            if fill_ratio < 1.0 {
                let arc_color = Color32::from_rgba_unmultiplied(0, 0, 0, 100);
                painter.circle_stroke(center, radius + 1.0, Stroke::new(1.0, arc_color));
            }

            // Selection highlight for resources
            if let EntitySelection::Resource(pos) = &state.selected {
                if pos.x == resource.position.x && pos.y == resource.position.y {
                    painter.circle_stroke(center, radius + 3.0, Stroke::new(2.0, Color32::YELLOW));
                }
            }
        }
    }

    // Draw buildings
    if state.map_layers.buildings {
        for building in &world.buildings {
            let screen_pos = world_to_screen(building.position.x, building.position.y, view_rect, state);
            let size = TILE_SIZE * state.map_zoom;

            if !view_rect.contains(screen_pos) {
                continue;
            }

            let building_rect = Rect::from_min_size(
                Pos2::new(screen_pos.x + size * 0.1, screen_pos.y + size * 0.1),
                Vec2::new(size * 0.8, size * 0.8),
            );

            let color = if building.completed {
                Color32::from_rgb(139, 90, 43)
            } else {
                Color32::from_rgb(180, 150, 100)
            };

            painter.rect_filled(building_rect, 2.0, color);

            // Construction progress bar
            if !building.completed {
                let bar_height = 3.0;
                let bar_rect = Rect::from_min_size(
                    Pos2::new(screen_pos.x, screen_pos.y + size - bar_height - 1.0),
                    Vec2::new(size * building.progress, bar_height),
                );
                painter.rect_filled(bar_rect, 0.0, Color32::from_rgb(100, 200, 100));
            }

            // Selection highlight for buildings
            if let EntitySelection::Building(pos) = &state.selected {
                if pos.x == building.position.x && pos.y == building.position.y {
                    painter.rect_stroke(
                        building_rect.expand(3.0),
                        2.0,
                        Stroke::new(2.0, Color32::YELLOW),
                    );
                }
            }
        }
    }

    // Draw agents
    if state.map_layers.agents {
        for agent in &snapshot.population.agents {
            if !agent.is_alive {
                continue;
            }

            if !should_show_agent(agent, &state.agent_filter) {
                continue;
            }

            let screen_pos = world_to_screen(agent.position.0, agent.position.1, view_rect, state);
            let size = TILE_SIZE * state.map_zoom;

            if !view_rect.contains(screen_pos) {
                continue;
            }

            let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);
            let radius = size * 0.4;

            // Color based on life stage
            let color = life_stage_color(agent.life_stage);
            painter.circle_filled(center, radius, color);

            // Health indicator ring
            if agent.health < 50.0 {
                let health_color = if agent.health < 25.0 {
                    Color32::RED
                } else {
                    Color32::YELLOW
                };
                painter.circle_stroke(center, radius + 1.0, Stroke::new(2.0, health_color));
            }

            // Energy indicator (small dot below agent when low)
            if agent.energy < 30.0 {
                let dot_pos = Pos2::new(center.x, center.y + radius + 4.0);
                painter.circle_filled(dot_pos, 2.0, Color32::from_rgb(100, 100, 255));
            }

            // Sleep indicator (Zzz when sleeping)
            if agent.is_sleeping {
                let z_color = Color32::from_rgb(138, 43, 226);
                let z_x = center.x + radius + 2.0;
                let z_y = center.y - radius;
                let font_size = (8.0 * state.map_zoom).max(6.0);
                painter.text(
                    Pos2::new(z_x, z_y),
                    egui::Align2::LEFT_CENTER,
                    "z",
                    egui::FontId::proportional(font_size),
                    z_color,
                );
                painter.text(
                    Pos2::new(z_x + font_size * 0.5, z_y - font_size * 0.4),
                    egui::Align2::LEFT_CENTER,
                    "z",
                    egui::FontId::proportional(font_size * 0.75),
                    z_color.gamma_multiply(0.7),
                );
            } else if agent.fatigue_severity > 0 {
                // Fatigue indicator (small dot on left side when tired but awake)
                let fatigue_color = match agent.fatigue_severity {
                    1 => Color32::from_rgb(200, 200, 100),
                    2 => Color32::from_rgb(255, 165, 0),
                    _ => Color32::from_rgb(255, 69, 0),
                };
                let indicator_pos = Pos2::new(center.x - radius - 3.0, center.y);
                painter.circle_filled(indicator_pos, 2.0, fatigue_color);
                if agent.fatigue_severity >= 3 {
                    painter.circle_stroke(indicator_pos, 3.5, Stroke::new(1.0, fatigue_color));
                }
            }

            // Drive urgency indicator (small colored triangle above agent)
            if let Some(drive) = agent.most_urgent_drive {
                let indicator_color = drive_color(drive);
                let top = Pos2::new(center.x, center.y - radius - 6.0);
                let left = Pos2::new(center.x - 3.0, center.y - radius - 2.0);
                let right = Pos2::new(center.x + 3.0, center.y - radius - 2.0);
                painter.add(egui::Shape::convex_polygon(
                    vec![top, left, right],
                    indicator_color,
                    Stroke::NONE,
                ));
            }

            // Selection highlight with animated pulse effect
            if let EntitySelection::Agent(selected_id) = &state.selected {
                if *selected_id == agent.id {
                    let time = ui.input(|i| i.time);
                    let pulse = (time * 3.0).sin() as f32 * 0.5 + 0.5;
                    let alpha = (150.0 + pulse * 105.0) as u8;
                    painter.circle_stroke(
                        center,
                        radius + 4.0 + pulse * 2.0,
                        Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, alpha)),
                    );
                }
            }
        }
    }

    // Draw selection highlight for terrain
    if let EntitySelection::Terrain(pos) = &state.selected {
        let screen_pos = world_to_screen(pos.x, pos.y, view_rect, state);
        let size = TILE_SIZE * state.map_zoom;
        let tile_rect = Rect::from_min_size(screen_pos, Vec2::new(size, size));
        painter.rect_stroke(tile_rect, 0.0, Stroke::new(2.0, Color32::WHITE));
    }

    // Handle clicks for selection - find what to select first
    let click_selection = if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (tile_x, tile_y) = screen_to_world(pos, view_rect, state);
            find_entity_at(snapshot, tile_x, tile_y)
        } else {
            None
        }
    } else {
        None
    };

    // Show tooltip on hover
    if let Some(pos) = response.hover_pos() {
        let (tile_x, tile_y) = screen_to_world(pos, view_rect, state);

        response.clone().on_hover_ui_at_pointer(|ui| {
            ui.set_max_width(260.0);

            // Show terrain header
            let terrain = world.tiles.iter()
                .find(|t| t.x == tile_x && t.y == tile_y)
                .map(|t| t.terrain);
            tooltip::render_terrain_header(ui, tile_x, tile_y, terrain);

            // Show ALL agents at this position
            let agents_here: Vec<_> = snapshot.population.agents.iter()
                .filter(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive)
                .collect();

            for agent in &agents_here {
                ui.separator();
                tooltip::render_agent_tooltip(ui, agent);
            }

            // Show ALL resources at this position
            let resources_here: Vec<_> = world.resources.iter()
                .filter(|r| r.position.x == tile_x && r.position.y == tile_y)
                .collect();

            for resource in &resources_here {
                ui.separator();
                tooltip::render_resource_tooltip(
                    ui,
                    resource.resource_type,
                    resource.amount,
                    resource.max_amount,
                );
            }

            // Show ALL buildings at this position
            let buildings_here: Vec<_> = world.buildings.iter()
                .filter(|b| b.position.x == tile_x && b.position.y == tile_y)
                .collect();

            for building in &buildings_here {
                ui.separator();
                tooltip::render_building_tooltip(
                    ui,
                    building.building_type,
                    building.completed,
                    building.progress,
                );
            }
        });
    }

    // Draw minimap and handle click
    let minimap_click = if state.show_minimap {
        draw_minimap(ui, &painter, view_rect, state, snapshot)
    } else {
        None
    };

    // Apply minimap click and selection
    if let Some((x, y)) = minimap_click {
        state.center_on_position(x, y, TILE_SIZE, (view_rect.width(), view_rect.height()));
    }

    // Apply click selection
    if let Some(selection) = click_selection {
        if let EntitySelection::Agent(id) = &selection {
            let _ = command_tx.send(SimulationCommand::SelectEntity(selection.clone()));
            if let Ok(mut request) = agent_data_request.lock() {
                *request = Some(*id);
            }
        }
        state.selected = selection;
    }

    // Map controls toolbar
    ui.add_space(5.0);
    render_map_controls(ui, state, view_rect);
}

/// Handle keyboard input for map navigation
fn handle_keyboard_input(ui: &mut Ui, state: &mut GuiState) {
    ui.input(|i| {
        let shift = i.modifiers.shift;
        let pan_amount = if shift { PAN_SPEED * 3.0 } else { PAN_SPEED };

        if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::A) {
            state.map_offset.0 -= pan_amount;
        }
        if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::D) {
            state.map_offset.0 += pan_amount;
        }
        if i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::W) {
            state.map_offset.1 -= pan_amount;
        }
        if i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::S) {
            state.map_offset.1 += pan_amount;
        }

        // Zoom with +/- keys
        if i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus) {
            state.map_zoom = (state.map_zoom + ZOOM_SPEED).min(MAX_ZOOM);
        }
        if i.key_pressed(Key::Minus) {
            state.map_zoom = (state.map_zoom - ZOOM_SPEED).max(MIN_ZOOM);
        }

        // Home key to reset view
        if i.key_pressed(Key::Home) {
            state.map_zoom = 1.0;
            state.map_offset = (0.0, 0.0);
        }

        // C key to center on selection
        if i.key_pressed(Key::C) {
            let view_size = (400.0, 300.0); // Approximate
            state.center_on_selected(TILE_SIZE, view_size);
        }

        // F key to toggle follow mode
        if i.key_pressed(Key::F) {
            state.follow_selected = !state.follow_selected;
        }

        // G key to toggle grid
        if i.key_pressed(Key::G) {
            state.map_layers.grid = !state.map_layers.grid;
        }

        // Escape to deselect
        if i.key_pressed(Key::Escape) {
            state.selected = EntitySelection::None;
            state.follow_selected = false;
        }
    });
}

/// Draw grid overlay
fn draw_grid(painter: &egui::Painter, view_rect: Rect, state: &GuiState, world_width: usize, world_height: usize) {
    let size = TILE_SIZE * state.map_zoom;
    let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 30);

    // Vertical lines
    for x in 0..=world_width {
        let screen_x = view_rect.min.x + x as f32 * size - state.map_offset.0;
        if screen_x >= view_rect.min.x && screen_x <= view_rect.max.x {
            painter.line_segment(
                [Pos2::new(screen_x, view_rect.min.y), Pos2::new(screen_x, view_rect.max.y)],
                Stroke::new(1.0, grid_color),
            );
        }
    }

    // Horizontal lines
    for y in 0..=world_height {
        let screen_y = view_rect.min.y + y as f32 * size - state.map_offset.1;
        if screen_y >= view_rect.min.y && screen_y <= view_rect.max.y {
            painter.line_segment(
                [Pos2::new(view_rect.min.x, screen_y), Pos2::new(view_rect.max.x, screen_y)],
                Stroke::new(1.0, grid_color),
            );
        }
    }
}

/// Draw minimap in corner, returns position to center on if clicked
fn draw_minimap(
    ui: &mut Ui,
    painter: &egui::Painter,
    view_rect: Rect,
    state: &GuiState,
    snapshot: &crate::gui::state::SimulationSnapshot,
) -> Option<(i32, i32)> {
    use crate::gui::state::MinimapPosition;

    let world = &snapshot.world;
    let minimap_size = state.minimap_settings.size;
    let opacity = (state.minimap_settings.opacity * 255.0) as u8;

    let minimap_pos = match state.minimap_settings.position {
        MinimapPosition::TopRight => Pos2::new(view_rect.max.x - minimap_size - 10.0, view_rect.min.y + 10.0),
        MinimapPosition::TopLeft => Pos2::new(view_rect.min.x + 10.0, view_rect.min.y + 10.0),
        MinimapPosition::BottomRight => Pos2::new(view_rect.max.x - minimap_size - 10.0, view_rect.max.y - minimap_size - 10.0),
        MinimapPosition::BottomLeft => Pos2::new(view_rect.min.x + 10.0, view_rect.max.y - minimap_size - 10.0),
    };

    let minimap_rect = Rect::from_min_size(minimap_pos, Vec2::new(minimap_size, minimap_size));

    // Background with configurable opacity
    painter.rect_filled(minimap_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, opacity));
    painter.rect_stroke(minimap_rect, 4.0, Stroke::new(1.0, Color32::from_rgb(100, 100, 100)));

    let scale_x = minimap_size / world.width as f32;
    let scale_y = minimap_size / world.height as f32;
    let scale = scale_x.min(scale_y);

    // Draw terrain (simplified)
    for tile in &world.tiles {
        let x = minimap_rect.min.x + tile.x as f32 * scale;
        let y = minimap_rect.min.y + tile.y as f32 * scale;
        let size = scale.max(1.0);

        let tile_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(size, size));
        let color = terrain_color(tile.terrain);
        painter.rect_filled(tile_rect, 0.0, color);
    }

    // Draw resources on minimap (if enabled)
    if state.minimap_settings.show_resources {
        for resource in &world.resources {
            let x = minimap_rect.min.x + resource.position.x as f32 * scale + scale / 2.0;
            let y = minimap_rect.min.y + resource.position.y as f32 * scale + scale / 2.0;
            let color = resource_color(resource.resource_type);
            painter.circle_filled(Pos2::new(x, y), 1.5, color);

            // Highlight selected resource
            if let crate::gui::state::EntitySelection::Resource(pos) = &state.selected {
                if pos.x == resource.position.x && pos.y == resource.position.y {
                    painter.circle_stroke(Pos2::new(x, y), 4.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw buildings on minimap (if enabled)
    if state.minimap_settings.show_buildings {
        for building in &world.buildings {
            let x = minimap_rect.min.x + building.position.x as f32 * scale;
            let y = minimap_rect.min.y + building.position.y as f32 * scale;
            let size = scale.max(2.0);
            let color = if building.completed {
                Color32::from_rgb(139, 90, 43)
            } else {
                Color32::from_rgb(180, 150, 100)
            };
            let building_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(size, size));
            painter.rect_filled(building_rect, 0.0, color);

            // Highlight selected building
            if let crate::gui::state::EntitySelection::Building(pos) = &state.selected {
                if pos.x == building.position.x && pos.y == building.position.y {
                    painter.rect_stroke(building_rect.expand(2.0), 0.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw agents as dots (if enabled)
    if state.minimap_settings.show_agents {
        for agent in &snapshot.population.agents {
            if !agent.is_alive {
                continue;
            }

            if !should_show_agent(agent, &state.agent_filter) {
                continue;
            }

            let x = minimap_rect.min.x + agent.position.0 as f32 * scale;
            let y = minimap_rect.min.y + agent.position.1 as f32 * scale;
            let color = life_stage_color(agent.life_stage);
            painter.circle_filled(Pos2::new(x + scale / 2.0, y + scale / 2.0), 2.0, color);

            // Highlight selected agent on minimap
            if let crate::gui::state::EntitySelection::Agent(selected_id) = &state.selected {
                if *selected_id == agent.id {
                    painter.circle_stroke(
                        Pos2::new(x + scale / 2.0, y + scale / 2.0),
                        4.0,
                        Stroke::new(1.5, Color32::WHITE),
                    );
                }
            }
        }
    }

    // Draw viewport rectangle (clamped to minimap bounds)
    let vp_x = minimap_rect.min.x + state.map_offset.0 / (TILE_SIZE * state.map_zoom) * scale;
    let vp_y = minimap_rect.min.y + state.map_offset.1 / (TILE_SIZE * state.map_zoom) * scale;
    let vp_w = (view_rect.width() / (TILE_SIZE * state.map_zoom)) * scale;
    let vp_h = (view_rect.height() / (TILE_SIZE * state.map_zoom)) * scale;

    let viewport_rect = Rect::from_min_size(Pos2::new(vp_x, vp_y), Vec2::new(vp_w, vp_h));
    let clamped_viewport = viewport_rect.intersect(minimap_rect);
    if clamped_viewport.width() > 0.0 && clamped_viewport.height() > 0.0 {
        painter.rect_stroke(clamped_viewport, 0.0, Stroke::new(2.0, Color32::WHITE));
    }

    // Minimap title and zoom indicator
    painter.text(
        Pos2::new(minimap_rect.min.x + 4.0, minimap_rect.min.y + 2.0),
        egui::Align2::LEFT_TOP,
        "Minimap",
        egui::FontId::proportional(10.0),
        Color32::from_rgba_unmultiplied(200, 200, 200, opacity),
    );
    painter.text(
        Pos2::new(minimap_rect.max.x - 4.0, minimap_rect.min.y + 2.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.0}%", state.map_zoom * 100.0),
        egui::FontId::proportional(9.0),
        Color32::from_rgba_unmultiplied(180, 180, 180, opacity),
    );

    // Click on minimap to pan - return position instead of modifying state
    let minimap_response = ui.interact(minimap_rect, ui.id().with("minimap"), Sense::click_and_drag());
    if minimap_response.clicked() || minimap_response.dragged() {
        if let Some(pos) = minimap_response.interact_pointer_pos() {
            let rel_x = (pos.x - minimap_rect.min.x) / scale;
            let rel_y = (pos.y - minimap_rect.min.y) / scale;
            return Some((rel_x as i32, rel_y as i32));
        }
    }

    None
}

/// Render map control toolbar
fn render_map_controls(ui: &mut Ui, state: &mut GuiState, view_rect: Rect) {
    ui.horizontal(|ui| {
        // Zoom controls
        ui.label("Zoom:");
        if ui.button("-").clicked() {
            state.map_zoom = (state.map_zoom - 0.25).max(MIN_ZOOM);
        }
        ui.label(format!("{:.0}%", state.map_zoom * 100.0));
        if ui.button("+").clicked() {
            state.map_zoom = (state.map_zoom + 0.25).min(MAX_ZOOM);
        }
        if ui.button("Reset").clicked() {
            state.map_zoom = 1.0;
            state.map_offset = (0.0, 0.0);
        }

        ui.separator();

        // Center on selection
        if ui.button("Center (C)").clicked() {
            state.center_on_selected(TILE_SIZE, (view_rect.width(), view_rect.height()));
        }

        // Follow mode toggle
        let follow_label = if state.follow_selected { "Following (F)" } else { "Follow (F)" };
        if ui.selectable_label(state.follow_selected, follow_label).clicked() {
            state.follow_selected = !state.follow_selected;
        }

        ui.separator();

        // Layer toggles
        ui.label("Layers:");
        ui.checkbox(&mut state.map_layers.terrain, "Terrain");
        ui.checkbox(&mut state.map_layers.resources, "Resources");
        ui.checkbox(&mut state.map_layers.buildings, "Buildings");
        ui.checkbox(&mut state.map_layers.agents, "Agents");
        ui.checkbox(&mut state.map_layers.grid, "Grid (G)");

        ui.separator();

        // Agent filter menu
        let filter_label = if state.agent_filter.is_filtering() {
            "Filter [ON]"
        } else {
            "Filter"
        };
        ui.menu_button(filter_label, |ui| {
            render_agent_filter_menu(ui, state);
        });

        ui.separator();

        // Minimap toggle with submenu
        ui.menu_button(if state.show_minimap { "Minimap [ON]" } else { "Minimap [OFF]" }, |ui| {
            ui.checkbox(&mut state.show_minimap, "Show Minimap (M)");
            ui.separator();
            ui.label(egui::RichText::new("Display").strong());
            ui.checkbox(&mut state.minimap_settings.show_resources, "Resources");
            ui.checkbox(&mut state.minimap_settings.show_buildings, "Buildings");
            ui.checkbox(&mut state.minimap_settings.show_agents, "Agents");
            ui.separator();
            ui.label(egui::RichText::new("Position").strong());
            ui.horizontal(|ui| {
                use crate::gui::state::MinimapPosition;
                if ui.selectable_label(state.minimap_settings.position == MinimapPosition::TopLeft, "TL").clicked() {
                    state.minimap_settings.position = MinimapPosition::TopLeft;
                }
                if ui.selectable_label(state.minimap_settings.position == MinimapPosition::TopRight, "TR").clicked() {
                    state.minimap_settings.position = MinimapPosition::TopRight;
                }
                if ui.selectable_label(state.minimap_settings.position == MinimapPosition::BottomLeft, "BL").clicked() {
                    state.minimap_settings.position = MinimapPosition::BottomLeft;
                }
                if ui.selectable_label(state.minimap_settings.position == MinimapPosition::BottomRight, "BR").clicked() {
                    state.minimap_settings.position = MinimapPosition::BottomRight;
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut state.minimap_settings.size, 80.0..=200.0).suffix("px"));
            });
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut state.minimap_settings.opacity, 0.3..=1.0));
            });
        });
    });
}

/// Convert world coordinates to screen position
fn world_to_screen(x: i32, y: i32, view_rect: Rect, state: &GuiState) -> Pos2 {
    Pos2::new(
        view_rect.min.x + x as f32 * TILE_SIZE * state.map_zoom - state.map_offset.0,
        view_rect.min.y + y as f32 * TILE_SIZE * state.map_zoom - state.map_offset.1,
    )
}

/// Convert screen position to world coordinates
fn screen_to_world(pos: Pos2, view_rect: Rect, state: &GuiState) -> (i32, i32) {
    let x = ((pos.x - view_rect.min.x + state.map_offset.0) / (TILE_SIZE * state.map_zoom)) as i32;
    let y = ((pos.y - view_rect.min.y + state.map_offset.1) / (TILE_SIZE * state.map_zoom)) as i32;
    (x, y)
}

/// Find entity at world position and return the selection
fn find_entity_at(
    snapshot: &crate::gui::state::SimulationSnapshot,
    tile_x: i32,
    tile_y: i32,
) -> Option<EntitySelection> {
    let world = &snapshot.world;

    // Priority: agents > buildings > resources > terrain
    if let Some(agent) = snapshot.population.agents.iter()
        .find(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive)
    {
        Some(EntitySelection::Agent(agent.id))
    } else if let Some(building) = world.buildings.iter()
        .find(|b| b.position.x == tile_x && b.position.y == tile_y)
    {
        Some(EntitySelection::Building(building.position))
    } else if let Some(resource) = world.resources.iter()
        .find(|r| r.position.x == tile_x && r.position.y == tile_y)
    {
        Some(EntitySelection::Resource(resource.position))
    } else {
        Some(EntitySelection::Terrain(crate::world::Position::new(tile_x, tile_y)))
    }
}

fn terrain_color(terrain: TerrainType) -> Color32 {
    match terrain {
        TerrainType::Plains => Color32::from_rgb(144, 238, 144),
        TerrainType::Meadow => Color32::from_rgb(124, 252, 0),
        TerrainType::Forest => Color32::from_rgb(34, 139, 34),
        TerrainType::Hills => Color32::from_rgb(139, 137, 112),
        TerrainType::Mountain => Color32::from_rgb(128, 128, 128),
        TerrainType::Water => Color32::from_rgb(65, 105, 225),
        TerrainType::Desert => Color32::from_rgb(238, 203, 173),
        TerrainType::Wetland => Color32::from_rgb(85, 107, 47),
        TerrainType::Beach => Color32::from_rgb(238, 214, 175),
        TerrainType::Sea => Color32::from_rgb(20, 60, 120),
        TerrainType::SaltMarsh => Color32::from_rgb(96, 128, 116),
        TerrainType::SaltFlat => Color32::from_rgb(232, 232, 224),
        TerrainType::Riverbank => Color32::from_rgb(107, 142, 35),
        TerrainType::Farmland => Color32::from_rgb(205, 170, 90),
    }
}

fn resource_color(resource_type: crate::world::ResourceType) -> Color32 {
    use crate::world::ResourceType;
    match resource_type {
        ResourceType::Wood => Color32::from_rgb(139, 69, 19),
        ResourceType::Stone => Color32::from_rgb(169, 169, 169),
        ResourceType::Iron => Color32::from_rgb(112, 128, 144),
        ResourceType::Food => Color32::from_rgb(255, 99, 71),
        ResourceType::Water => Color32::from_rgb(0, 191, 255),
        ResourceType::Coal => Color32::from_rgb(47, 79, 79),
        ResourceType::Grain => Color32::from_rgb(255, 215, 0),
        ResourceType::Herbs => Color32::from_rgb(0, 128, 0),
        _ => Color32::from_rgb(200, 200, 200),
    }
}

fn life_stage_color(life_stage: LifeStage) -> Color32 {
    match life_stage {
        LifeStage::Infant => Color32::from_rgb(255, 182, 193),
        LifeStage::Child => Color32::from_rgb(135, 206, 250),
        LifeStage::Adolescent => Color32::from_rgb(144, 238, 144),
        LifeStage::Adult => Color32::from_rgb(255, 255, 255),
        LifeStage::Elderly => Color32::from_rgb(192, 192, 192),
    }
}

fn drive_color(drive: crate::core::DriveType) -> Color32 {
    use crate::core::DriveType;
    match drive {
        DriveType::Hunger => Color32::from_rgb(255, 140, 0),
        DriveType::Thirst => Color32::from_rgb(0, 191, 255),
        DriveType::Rest => Color32::from_rgb(138, 43, 226),
        DriveType::Safety => Color32::from_rgb(255, 0, 0),
        DriveType::Aggression => Color32::from_rgb(200, 40, 0),
        DriveType::Social => Color32::from_rgb(255, 105, 180),
        DriveType::Shelter => Color32::from_rgb(139, 90, 43),
        DriveType::Curiosity => Color32::from_rgb(255, 255, 0),
        DriveType::Preparedness => Color32::from_rgb(218, 165, 32),
        DriveType::Industry => Color32::from_rgb(169, 169, 169),
        DriveType::Sustenance => Color32::from_rgb(144, 238, 144),
        DriveType::Reproduction => Color32::from_rgb(255, 182, 193),
        DriveType::Luxury => Color32::from_rgb(230, 230, 250),
        DriveType::Utility => Color32::from_rgb(192, 192, 192),
        DriveType::Construction => Color32::from_rgb(139, 69, 19),
        DriveType::Protection => Color32::from_rgb(255, 215, 0),
    }
}

fn should_show_agent(agent: &AgentSnapshot, filter: &AgentMapFilter) -> bool {
    if !filter.show_life_stage(agent.life_stage) {
        return false;
    }

    if agent.is_sleeping {
        return filter.show_sleeping;
    }

    filter.show_job(agent.inferred_job)
}

fn render_agent_filter_menu(ui: &mut egui::Ui, state: &mut GuiState) {
    if ui.button("Reset All").clicked() {
        state.agent_filter.reset();
    }

    ui.separator();

    ui.label(egui::RichText::new("Life Stage").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.agent_filter.show_infant, "Infant");
        ui.checkbox(&mut state.agent_filter.show_child, "Child");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.agent_filter.show_adolescent, "Adolescent");
        ui.checkbox(&mut state.agent_filter.show_adult, "Adult");
    });
    ui.checkbox(&mut state.agent_filter.show_elderly, "Elderly");

    ui.separator();

    ui.label(egui::RichText::new("Gender").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.agent_filter.show_male, "Male");
        ui.checkbox(&mut state.agent_filter.show_female, "Female");
    });

    ui.separator();

    ui.label(egui::RichText::new("Status").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.agent_filter.show_sleeping, "Sleeping");
        ui.checkbox(&mut state.agent_filter.show_idle, "Idle");
    });

    ui.separator();

    ui.label(egui::RichText::new("Activity").strong());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.checkbox(&mut state.agent_filter.show_gathering, "Gathering");
            ui.checkbox(&mut state.agent_filter.show_farming, "Farming");
            ui.checkbox(&mut state.agent_filter.show_hunting, "Hunting");
            ui.checkbox(&mut state.agent_filter.show_fishing, "Fishing");
            ui.checkbox(&mut state.agent_filter.show_mining, "Mining");
            ui.checkbox(&mut state.agent_filter.show_cooking, "Cooking");
        });
        ui.vertical(|ui| {
            ui.checkbox(&mut state.agent_filter.show_building, "Building");
            ui.checkbox(&mut state.agent_filter.show_crafting, "Crafting");
            ui.checkbox(&mut state.agent_filter.show_exploring, "Exploring");
            ui.checkbox(&mut state.agent_filter.show_social, "Social");
            ui.checkbox(&mut state.agent_filter.show_caretaking, "Caretaking");
            ui.checkbox(&mut state.agent_filter.show_labor, "Labor");
        });
    });
}
