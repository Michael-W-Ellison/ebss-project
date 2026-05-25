// src/bevy_gui/ui/map.rs
//! Interactive world map rendering with camera controls.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::bevy_gui::resources::{
    CurrentSnapshot, MapViewState, Selection, EntitySelection,
    PanelVisibility, NotificationQueue,
};
use crate::bevy_gui::events::{SimulationCommand, CenterMapRequest};
use crate::gui::state::{AgentSnapshot, SimulationSnapshot};
use crate::world::Position;

use super::tooltip;

const TILE_SIZE: f32 = 12.0;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 4.0;

/// Main map rendering system
pub fn render_map(
    mut egui_ctx: EguiContexts,
    snapshot: Res<CurrentSnapshot>,
    mut map_view: ResMut<MapViewState>,
    mut selection: ResMut<Selection>,
    mut sim_commands: EventWriter<SimulationCommand>,
    mut center_request: EventWriter<CenterMapRequest>,
    mut notifications: ResMut<NotificationQueue>,
    _panels: Res<PanelVisibility>,
    time: Res<Time>,
) {
    let Some(snap) = &snapshot.snapshot else {
        egui::CentralPanel::default().show(egui_ctx.ctx_mut(), |ui| {
            ui.centered_and_justified(|ui| {
                ui.heading("Waiting for simulation data...");
            });
        });
        return;
    };

    let current_time = time.elapsed_secs_f64();

    egui::CentralPanel::default().show(egui_ctx.ctx_mut(), |ui| {
        let available_size = ui.available_size();
        let world = &snap.world;

        let map_width = world.width as f32 * TILE_SIZE * map_view.zoom;
        let map_height = world.height as f32 * TILE_SIZE * map_view.zoom;

        // Allocate painter for map area (leave room for controls)
        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_size.x, available_size.y - 60.0),
            Sense::click_and_drag(),
        );

        let view_rect = response.rect;

        // Handle drag to pan
        if response.dragged() {
            let delta = response.drag_delta();
            map_view.offset.0 -= delta.x;
            map_view.offset.1 -= delta.y;
        }

        // Handle zoom with scroll wheel (zoom toward cursor)
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && response.hovered() {
            if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let old_zoom = map_view.zoom;
                let zoom_factor = 1.0 + scroll_delta * 0.002;
                map_view.zoom = (map_view.zoom * zoom_factor).clamp(MIN_ZOOM, MAX_ZOOM);

                // Adjust offset to zoom toward cursor position
                let cursor_rel = (cursor_pos.x - view_rect.min.x, cursor_pos.y - view_rect.min.y);
                let world_x = map_view.offset.0 + cursor_rel.0;
                let world_y = map_view.offset.1 + cursor_rel.1;
                let scale_change = map_view.zoom / old_zoom;
                map_view.offset.0 = world_x * scale_change - cursor_rel.0;
                map_view.offset.1 = world_y * scale_change - cursor_rel.1;
            }
        }

        // Clamp offset to keep map in view
        let max_offset_x = (map_width - view_rect.width()).max(0.0);
        let max_offset_y = (map_height - view_rect.height()).max(0.0);
        map_view.offset.0 = map_view.offset.0.clamp(-view_rect.width() * 0.5, max_offset_x + view_rect.width() * 0.5);
        map_view.offset.1 = map_view.offset.1.clamp(-view_rect.height() * 0.5, max_offset_y + view_rect.height() * 0.5);

        // Draw background
        painter.rect_filled(view_rect, 0.0, Color32::from_rgb(20, 20, 30));

        // Calculate visible tile range for culling
        let start_tile_x = ((map_view.offset.0 / (TILE_SIZE * map_view.zoom)).floor() as i32).max(0);
        let start_tile_y = ((map_view.offset.1 / (TILE_SIZE * map_view.zoom)).floor() as i32).max(0);
        let end_tile_x = (((map_view.offset.0 + view_rect.width()) / (TILE_SIZE * map_view.zoom)).ceil() as i32 + 1).min(world.width as i32);
        let end_tile_y = (((map_view.offset.1 + view_rect.height()) / (TILE_SIZE * map_view.zoom)).ceil() as i32 + 1).min(world.height as i32);

        // Draw terrain tiles (with culling)
        if map_view.layers.terrain {
            for tile in &world.tiles {
                if tile.x < start_tile_x || tile.x >= end_tile_x || tile.y < start_tile_y || tile.y >= end_tile_y {
                    continue;
                }

                let screen_pos = world_to_screen(tile.x, tile.y, view_rect, &map_view);
                let size = TILE_SIZE * map_view.zoom;

                let tile_rect = Rect::from_min_size(screen_pos, Vec2::new(size, size));
                if !view_rect.intersects(tile_rect) {
                    continue;
                }

                let color = tooltip::terrain_color(tile.terrain);
                painter.rect_filled(tile_rect, 0.0, color);
            }
        }

        // Draw grid overlay
        if map_view.layers.grid {
            draw_grid(&painter, view_rect, &map_view, world.width, world.height);
        }

        // Draw resources
        if map_view.layers.resources {
            for resource in &world.resources {
                let screen_pos = world_to_screen(resource.position.x, resource.position.y, view_rect, &map_view);
                let size = TILE_SIZE * map_view.zoom;

                if !view_rect.contains(screen_pos) {
                    continue;
                }

                let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);
                let radius = size * 0.3;

                let color = tooltip::resource_color(resource.resource_type);
                painter.circle_filled(center, radius, color);

                // Resource amount indicator (small arc)
                let fill_ratio = resource.amount as f32 / resource.max_amount.max(1) as f32;
                if fill_ratio < 1.0 {
                    let arc_color = Color32::from_rgba_unmultiplied(0, 0, 0, 100);
                    painter.circle_stroke(center, radius + 1.0, Stroke::new(1.0, arc_color));
                }

                // Selection highlight for resources
                if let EntitySelection::Resource(pos) = &selection.current {
                    if pos.x == resource.position.x && pos.y == resource.position.y {
                        painter.circle_stroke(center, radius + 3.0, Stroke::new(2.0, Color32::YELLOW));
                    }
                }
            }
        }

        // Draw buildings
        if map_view.layers.buildings {
            for building in &world.buildings {
                let screen_pos = world_to_screen(building.position.x, building.position.y, view_rect, &map_view);
                let size = TILE_SIZE * map_view.zoom;

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
                if let EntitySelection::Building(pos) = &selection.current {
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
        if map_view.layers.agents {
            for agent in &snap.population.agents {
                if !agent.is_alive {
                    continue;
                }

                if !should_show_agent(agent, &map_view) {
                    continue;
                }

                let screen_pos = world_to_screen(agent.position.0, agent.position.1, view_rect, &map_view);
                let size = TILE_SIZE * map_view.zoom;

                if !view_rect.contains(screen_pos) {
                    continue;
                }

                let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);
                let radius = size * 0.4;

                // Color based on life stage
                let color = tooltip::life_stage_map_color(agent.life_stage);
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
                    let font_size = (8.0 * map_view.zoom).max(6.0);
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
                    let indicator_color = tooltip::drive_color(drive);
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
                if let EntitySelection::Agent(selected_id) = &selection.current {
                    if *selected_id == agent.id {
                        let pulse = (current_time * 3.0).sin() as f32 * 0.5 + 0.5;
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
        if let EntitySelection::Terrain(pos) = &selection.current {
            let screen_pos = world_to_screen(pos.x, pos.y, view_rect, &map_view);
            let size = TILE_SIZE * map_view.zoom;
            let tile_rect = Rect::from_min_size(screen_pos, Vec2::new(size, size));
            painter.rect_stroke(tile_rect, 0.0, Stroke::new(2.0, Color32::WHITE));
        }

        // Handle clicks for selection - find what to select first
        let click_selection = if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let (tile_x, tile_y) = screen_to_world(pos, view_rect, &map_view);
                find_entity_at(snap, tile_x, tile_y)
            } else {
                None
            }
        } else {
            None
        };

        // Show tooltip on hover
        if let Some(pos) = response.hover_pos() {
            let (tile_x, tile_y) = screen_to_world(pos, view_rect, &map_view);

            response.clone().on_hover_ui_at_pointer(|ui| {
                ui.set_max_width(260.0);

                // Show terrain header
                let terrain = world.tiles.iter()
                    .find(|t| t.x == tile_x && t.y == tile_y)
                    .map(|t| t.terrain);
                tooltip::render_terrain_header(ui, tile_x, tile_y, terrain);

                // Show ALL agents at this position
                let agents_here: Vec<_> = snap.population.agents.iter()
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
        let minimap_click = if map_view.minimap.enabled {
            draw_minimap(&painter, view_rect, &map_view, snap, &selection, current_time, ui)
        } else {
            None
        };

        // Apply minimap click
        if let Some((x, y)) = minimap_click {
            center_on_tile(&mut map_view, x, y, view_rect);
        }

        // Apply click selection
        if let Some(new_selection) = click_selection {
            if let EntitySelection::Agent(id) = &new_selection {
                sim_commands.send(SimulationCommand::SelectEntity(
                    crate::bevy_gui::resources::EntitySelection::Agent(*id)
                ));
            }
            selection.current = new_selection;
        }

        // Map controls toolbar
        ui.add_space(5.0);
        render_map_controls(ui, &mut map_view, &mut selection, &mut center_request, &mut notifications, snap, current_time);
    });
}

/// Draw grid overlay
fn draw_grid(painter: &egui::Painter, view_rect: Rect, map_view: &MapViewState, world_width: usize, world_height: usize) {
    let size = TILE_SIZE * map_view.zoom;
    let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 30);

    // Vertical lines
    for x in 0..=world_width {
        let screen_x = view_rect.min.x + x as f32 * size - map_view.offset.0;
        if screen_x >= view_rect.min.x && screen_x <= view_rect.max.x {
            painter.line_segment(
                [Pos2::new(screen_x, view_rect.min.y), Pos2::new(screen_x, view_rect.max.y)],
                Stroke::new(1.0, grid_color),
            );
        }
    }

    // Horizontal lines
    for y in 0..=world_height {
        let screen_y = view_rect.min.y + y as f32 * size - map_view.offset.1;
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
    painter: &egui::Painter,
    view_rect: Rect,
    map_view: &MapViewState,
    snapshot: &SimulationSnapshot,
    selection: &Selection,
    _current_time: f64,
    ui: &mut egui::Ui,
) -> Option<(i32, i32)> {
    use crate::bevy_gui::resources::MinimapPosition;

    let world = &snapshot.world;
    let minimap_size = map_view.minimap.size;
    let opacity = (map_view.minimap.opacity * 255.0) as u8;

    let minimap_pos = match map_view.minimap.position {
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
        let color = tooltip::terrain_color(tile.terrain);
        painter.rect_filled(tile_rect, 0.0, color);
    }

    // Draw resources on minimap (if enabled)
    if map_view.minimap.show_resources {
        for resource in &world.resources {
            let x = minimap_rect.min.x + resource.position.x as f32 * scale + scale / 2.0;
            let y = minimap_rect.min.y + resource.position.y as f32 * scale + scale / 2.0;
            let color = tooltip::resource_color(resource.resource_type);
            painter.circle_filled(Pos2::new(x, y), 1.5, color);

            // Highlight selected resource
            if let EntitySelection::Resource(pos) = &selection.current {
                if pos.x == resource.position.x && pos.y == resource.position.y {
                    painter.circle_stroke(Pos2::new(x, y), 4.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw buildings on minimap (if enabled)
    if map_view.minimap.show_buildings {
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
            if let EntitySelection::Building(pos) = &selection.current {
                if pos.x == building.position.x && pos.y == building.position.y {
                    painter.rect_stroke(building_rect.expand(2.0), 0.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw agents as dots (if enabled)
    if map_view.minimap.show_agents {
        for agent in &snapshot.population.agents {
            if !agent.is_alive {
                continue;
            }

            if !should_show_agent(agent, map_view) {
                continue;
            }

            let x = minimap_rect.min.x + agent.position.0 as f32 * scale;
            let y = minimap_rect.min.y + agent.position.1 as f32 * scale;
            let color = tooltip::life_stage_map_color(agent.life_stage);
            painter.circle_filled(Pos2::new(x + scale / 2.0, y + scale / 2.0), 2.0, color);

            // Highlight selected agent on minimap
            if let EntitySelection::Agent(selected_id) = &selection.current {
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
    let vp_x = minimap_rect.min.x + map_view.offset.0 / (TILE_SIZE * map_view.zoom) * scale;
    let vp_y = minimap_rect.min.y + map_view.offset.1 / (TILE_SIZE * map_view.zoom) * scale;
    let vp_w = (view_rect.width() / (TILE_SIZE * map_view.zoom)) * scale;
    let vp_h = (view_rect.height() / (TILE_SIZE * map_view.zoom)) * scale;

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
        format!("{:.0}%", map_view.zoom * 100.0),
        egui::FontId::proportional(9.0),
        Color32::from_rgba_unmultiplied(180, 180, 180, opacity),
    );

    // Click on minimap to pan
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
fn render_map_controls(
    ui: &mut egui::Ui,
    map_view: &mut MapViewState,
    selection: &mut Selection,
    center_request: &mut EventWriter<CenterMapRequest>,
    notifications: &mut NotificationQueue,
    snapshot: &SimulationSnapshot,
    current_time: f64,
) {
    ui.horizontal(|ui| {
        // Zoom controls
        ui.label("Zoom:");
        if ui.button("-").clicked() {
            map_view.zoom = (map_view.zoom - 0.25).max(MIN_ZOOM);
        }
        ui.label(format!("{:.0}%", map_view.zoom * 100.0));
        if ui.button("+").clicked() {
            map_view.zoom = (map_view.zoom + 0.25).min(MAX_ZOOM);
        }
        if ui.button("Reset").clicked() {
            map_view.reset_view();
        }

        ui.separator();

        // Center on selection
        if ui.button("Center (C)").clicked() {
            if let EntitySelection::Agent(id) = &selection.current {
                if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == *id && a.is_alive) {
                    center_request.send(CenterMapRequest { x: agent.position.0, y: agent.position.1 });
                    notifications.info("Centering on selection", current_time);
                }
            }
        }

        // Follow mode toggle
        let follow_label = if selection.follow_selected { "Following (F)" } else { "Follow (F)" };
        if ui.selectable_label(selection.follow_selected, follow_label).clicked() {
            selection.toggle_follow();
            if selection.follow_selected {
                notifications.info("Follow mode enabled", current_time);
            }
        }

        ui.separator();

        // Layer toggles
        ui.label("Layers:");
        ui.checkbox(&mut map_view.layers.terrain, "Terrain");
        ui.checkbox(&mut map_view.layers.resources, "Resources");
        ui.checkbox(&mut map_view.layers.buildings, "Buildings");
        ui.checkbox(&mut map_view.layers.agents, "Agents");
        ui.checkbox(&mut map_view.layers.grid, "Grid (G)");

        ui.separator();

        // Agent filter menu
        let filter_label = if map_view.agent_filter.is_filtering() {
            "Filter [ON]"
        } else {
            "Filter"
        };
        ui.menu_button(filter_label, |ui| {
            render_agent_filter_menu(ui, map_view);
        });

        ui.separator();

        // Minimap toggle with submenu
        ui.menu_button(if map_view.minimap.enabled { "Minimap [ON]" } else { "Minimap [OFF]" }, |ui| {
            ui.checkbox(&mut map_view.minimap.enabled, "Show Minimap (M)");
            ui.separator();
            ui.label(egui::RichText::new("Display").strong());
            ui.checkbox(&mut map_view.minimap.show_resources, "Resources");
            ui.checkbox(&mut map_view.minimap.show_buildings, "Buildings");
            ui.checkbox(&mut map_view.minimap.show_agents, "Agents");
            ui.separator();
            ui.label(egui::RichText::new("Position").strong());
            ui.horizontal(|ui| {
                use crate::bevy_gui::resources::MinimapPosition;
                if ui.selectable_label(map_view.minimap.position == MinimapPosition::TopLeft, "TL").clicked() {
                    map_view.minimap.position = MinimapPosition::TopLeft;
                }
                if ui.selectable_label(map_view.minimap.position == MinimapPosition::TopRight, "TR").clicked() {
                    map_view.minimap.position = MinimapPosition::TopRight;
                }
                if ui.selectable_label(map_view.minimap.position == MinimapPosition::BottomLeft, "BL").clicked() {
                    map_view.minimap.position = MinimapPosition::BottomLeft;
                }
                if ui.selectable_label(map_view.minimap.position == MinimapPosition::BottomRight, "BR").clicked() {
                    map_view.minimap.position = MinimapPosition::BottomRight;
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut map_view.minimap.size, 80.0..=200.0).suffix("px"));
            });
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut map_view.minimap.opacity, 0.3..=1.0));
            });
        });
    });
}

/// Render agent filter menu
fn render_agent_filter_menu(ui: &mut egui::Ui, map_view: &mut MapViewState) {
    if ui.button("Reset All").clicked() {
        map_view.agent_filter.reset();
    }

    ui.separator();

    ui.label(egui::RichText::new("Life Stage").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut map_view.agent_filter.show_infant, "Infant");
        ui.checkbox(&mut map_view.agent_filter.show_child, "Child");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut map_view.agent_filter.show_adolescent, "Adolescent");
        ui.checkbox(&mut map_view.agent_filter.show_adult, "Adult");
    });
    ui.checkbox(&mut map_view.agent_filter.show_elderly, "Elderly");

    ui.separator();

    ui.label(egui::RichText::new("Gender").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut map_view.agent_filter.show_male, "Male");
        ui.checkbox(&mut map_view.agent_filter.show_female, "Female");
    });

    ui.separator();

    ui.label(egui::RichText::new("Status").strong());
    ui.horizontal(|ui| {
        ui.checkbox(&mut map_view.agent_filter.show_sleeping, "Sleeping");
        ui.checkbox(&mut map_view.agent_filter.show_idle, "Idle");
    });

    ui.separator();

    ui.label(egui::RichText::new("Activity").strong());
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.checkbox(&mut map_view.agent_filter.show_gathering, "Gathering");
            ui.checkbox(&mut map_view.agent_filter.show_farming, "Farming");
            ui.checkbox(&mut map_view.agent_filter.show_hunting, "Hunting");
            ui.checkbox(&mut map_view.agent_filter.show_fishing, "Fishing");
            ui.checkbox(&mut map_view.agent_filter.show_mining, "Mining");
            ui.checkbox(&mut map_view.agent_filter.show_cooking, "Cooking");
        });
        ui.vertical(|ui| {
            ui.checkbox(&mut map_view.agent_filter.show_building, "Building");
            ui.checkbox(&mut map_view.agent_filter.show_crafting, "Crafting");
            ui.checkbox(&mut map_view.agent_filter.show_exploring, "Exploring");
            ui.checkbox(&mut map_view.agent_filter.show_social, "Social");
            ui.checkbox(&mut map_view.agent_filter.show_caretaking, "Caretaking");
            ui.checkbox(&mut map_view.agent_filter.show_labor, "Labor");
        });
    });
}

/// Convert world coordinates to screen position
fn world_to_screen(x: i32, y: i32, view_rect: Rect, map_view: &MapViewState) -> Pos2 {
    Pos2::new(
        view_rect.min.x + x as f32 * TILE_SIZE * map_view.zoom - map_view.offset.0,
        view_rect.min.y + y as f32 * TILE_SIZE * map_view.zoom - map_view.offset.1,
    )
}

/// Convert screen position to world coordinates
fn screen_to_world(pos: Pos2, view_rect: Rect, map_view: &MapViewState) -> (i32, i32) {
    let x = ((pos.x - view_rect.min.x + map_view.offset.0) / (TILE_SIZE * map_view.zoom)) as i32;
    let y = ((pos.y - view_rect.min.y + map_view.offset.1) / (TILE_SIZE * map_view.zoom)) as i32;
    (x, y)
}

/// Center map view on a specific tile
fn center_on_tile(map_view: &mut MapViewState, tile_x: i32, tile_y: i32, view_rect: Rect) {
    let world_x = tile_x as f32 * TILE_SIZE * map_view.zoom;
    let world_y = tile_y as f32 * TILE_SIZE * map_view.zoom;
    map_view.offset.0 = world_x - view_rect.width() / 2.0;
    map_view.offset.1 = world_y - view_rect.height() / 2.0;
}

/// Find entity at world position and return the selection
fn find_entity_at(
    snapshot: &SimulationSnapshot,
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
        Some(EntitySelection::Terrain(Position::new(tile_x, tile_y)))
    }
}

/// Check if agent should be shown based on current filters
fn should_show_agent(agent: &AgentSnapshot, map_view: &MapViewState) -> bool {
    let filter = &map_view.agent_filter;

    if !filter.show_life_stage(agent.life_stage) {
        return false;
    }

    if !filter.show_gender(agent.gender) {
        return false;
    }

    if agent.is_sleeping {
        return filter.show_sleeping;
    }

    filter.show_job(agent.inferred_job)
}
