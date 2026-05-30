// src/bevy_gui/ui/map.rs
//! Interactive world map rendering with camera controls and hover tooltips.
//!
//! Mouse input features:
//! - Drag-to-pan: Click and drag to pan the map view
//! - Scroll-to-zoom: Mouse wheel zooms toward cursor position
//! - Offset clamping: View is constrained to stay within map bounds with margin

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

use crate::bevy_gui::resources::{
    CurrentSnapshot, MapViewState, Selection, EntitySelection,
    PanelVisibility, NotificationQueue, MinimapPosition,
};
use crate::bevy_gui::events::{SimulationCommand, CenterMapRequest};
use crate::gui::state::{AgentSnapshot, SimulationSnapshot};
use crate::world::Position;

use super::tooltip;

const TILE_SIZE: f32 = 12.0;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 4.0;
const ZOOM_SPEED: f32 = 0.002;
const VIEW_MARGIN_FACTOR: f32 = 0.5;

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

        // Calculate map dimensions in screen space
        let map_width = world.width as f32 * TILE_SIZE * map_view.zoom;
        let map_height = world.height as f32 * TILE_SIZE * map_view.zoom;

        // Allocate map area (leave room for toolbar at bottom)
        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_size.x, available_size.y - 35.0),
            Sense::click_and_drag(),
        );
        let view_rect = response.rect;

        // === DRAG-TO-PAN ===
        // When dragging, move the view offset opposite to the drag direction
        if response.dragged() {
            let delta = response.drag_delta();
            map_view.offset.0 -= delta.x;
            map_view.offset.1 -= delta.y;
        }

        // === SCROLL-TO-ZOOM (toward cursor) ===
        // When scrolling, zoom in/out centered on the cursor position
        // This creates a natural zoom experience where the point under the cursor stays fixed
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && response.hovered() {
            if let Some(cursor_pos) = ui.input(|i| i.pointer.hover_pos()) {
                // Calculate cursor position relative to view
                let cursor_rel_x = cursor_pos.x - view_rect.min.x;
                let cursor_rel_y = cursor_pos.y - view_rect.min.y;

                // Calculate world position under cursor before zoom
                let world_x = map_view.offset.0 + cursor_rel_x;
                let world_y = map_view.offset.1 + cursor_rel_y;

                // Apply zoom with smooth acceleration
                let old_zoom = map_view.zoom;
                let zoom_factor = 1.0 + scroll_delta * ZOOM_SPEED;
                let new_zoom = (map_view.zoom * zoom_factor).clamp(MIN_ZOOM, MAX_ZOOM);
                map_view.zoom = new_zoom;

                // Adjust offset so cursor stays over the same world position
                // world_pos = offset + cursor_rel => offset = world_pos - cursor_rel
                // After zoom: new_world_pos = old_world_pos * (new_zoom / old_zoom)
                let scale_change = new_zoom / old_zoom;
                map_view.offset.0 = world_x * scale_change - cursor_rel_x;
                map_view.offset.1 = world_y * scale_change - cursor_rel_y;
            }
        }

        // === OFFSET CLAMPING ===
        // Constrain the view to stay within map bounds with some margin
        // This prevents scrolling into empty space while allowing some overshoot for edge visibility
        let view_margin_x = view_rect.width() * VIEW_MARGIN_FACTOR;
        let view_margin_y = view_rect.height() * VIEW_MARGIN_FACTOR;

        // Calculate clamping bounds
        // Min offset allows scrolling half a view width into negative space
        // Max offset allows scrolling to see the far edge of the map with margin
        let min_offset_x = -view_margin_x;
        let min_offset_y = -view_margin_y;
        let max_offset_x = (map_width - view_rect.width() + view_margin_x).max(0.0);
        let max_offset_y = (map_height - view_rect.height() + view_margin_y).max(0.0);

        map_view.offset.0 = map_view.offset.0.clamp(min_offset_x, max_offset_x);
        map_view.offset.1 = map_view.offset.1.clamp(min_offset_y, max_offset_y);

        // Calculate visible tile range for culling
        let start_tile_x = ((map_view.offset.0 / (TILE_SIZE * map_view.zoom)).floor() as i32).max(0);
        let start_tile_y = ((map_view.offset.1 / (TILE_SIZE * map_view.zoom)).floor() as i32).max(0);
        let end_tile_x = (((map_view.offset.0 + view_rect.width()) / (TILE_SIZE * map_view.zoom)).ceil() as i32 + 1).min(world.width as i32);
        let end_tile_y = (((map_view.offset.1 + view_rect.height()) / (TILE_SIZE * map_view.zoom)).ceil() as i32 + 1).min(world.height as i32);

        // Draw background
        painter.rect_filled(view_rect, 0.0, Color32::from_rgb(20, 25, 35));

        let size = TILE_SIZE * map_view.zoom;

        // Draw terrain tiles (with culling)
        if map_view.layers.terrain {
            for tile in &world.tiles {
                if tile.x < start_tile_x || tile.x >= end_tile_x || tile.y < start_tile_y || tile.y >= end_tile_y {
                    continue;
                }
                let screen_pos = world_to_screen(tile.x, tile.y, view_rect, &map_view);
                let tile_rect = Rect::from_min_size(screen_pos, Vec2::new(size, size));

                if tile_rect.intersects(view_rect) {
                    let color = tooltip::terrain_color(tile.terrain);
                    painter.rect_filled(tile_rect, 0.0, color);
                }
            }
        }

        // Draw grid overlay
        if map_view.layers.grid {
            draw_grid(&painter, view_rect, &map_view, world.width, world.height);
        }

        // Draw resources
        if map_view.layers.resources {
            for resource in &world.resources {
                let x = resource.position.x;
                let y = resource.position.y;
                if x < start_tile_x || x >= end_tile_x || y < start_tile_y || y >= end_tile_y {
                    continue;
                }
                let screen_pos = world_to_screen(x, y, view_rect, &map_view);
                let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);

                if view_rect.contains(center) {
                    let color = tooltip::resource_color(resource.resource_type);
                    let radius = (size / 4.0).max(2.0);
                    painter.circle_filled(center, radius, color);

                    // Selection highlight with animated pulse
                    if let EntitySelection::Resource(pos) = &selection.current {
                        if pos.x == x && pos.y == y {
                            let pulse = (current_time * 3.0).sin() as f32 * 0.3 + 0.7;
                            painter.circle_stroke(center, radius + 3.0, Stroke::new(2.0 * pulse, Color32::YELLOW));
                        }
                    }
                }
            }
        }

        // Draw buildings
        if map_view.layers.buildings {
            for building in &world.buildings {
                let x = building.position.x;
                let y = building.position.y;
                if x < start_tile_x || x >= end_tile_x || y < start_tile_y || y >= end_tile_y {
                    continue;
                }
                let screen_pos = world_to_screen(x, y, view_rect, &map_view);
                let building_size = size * 0.8;
                let building_rect = Rect::from_min_size(
                    Pos2::new(screen_pos.x + size * 0.1, screen_pos.y + size * 0.1),
                    Vec2::new(building_size, building_size),
                );

                if building_rect.intersects(view_rect) {
                    let color = tooltip::building_color(building.building_type, building.completed);
                    painter.rect_filled(building_rect, 2.0, color);

                    // Progress indicator for incomplete buildings
                    if !building.completed {
                        let progress_height = building_size * building.progress;
                        let progress_rect = Rect::from_min_size(
                            Pos2::new(building_rect.min.x, building_rect.max.y - progress_height),
                            Vec2::new(building_size, progress_height),
                        );
                        painter.rect_filled(progress_rect, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 60));
                    }

                    // Selection highlight
                    if let EntitySelection::Building(pos) = &selection.current {
                        if pos.x == x && pos.y == y {
                            let pulse = (current_time * 3.0).sin() as f32 * 0.3 + 0.7;
                            painter.rect_stroke(building_rect.expand(3.0), 2.0, Stroke::new(2.0 * pulse, Color32::YELLOW));
                        }
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

                let x = agent.position.0;
                let y = agent.position.1;
                if x < start_tile_x || x >= end_tile_x || y < start_tile_y || y >= end_tile_y {
                    continue;
                }

                let screen_pos = world_to_screen(x, y, view_rect, &map_view);
                let center = Pos2::new(screen_pos.x + size / 2.0, screen_pos.y + size / 2.0);

                if view_rect.contains(center) {
                    let color = tooltip::life_stage_map_color(agent.life_stage);
                    let radius = (size / 3.0).max(3.0);

                    // Draw agent with different style for sleeping
                    if agent.is_sleeping {
                        painter.circle_filled(center, radius, Color32::from_rgb(80, 60, 120));
                        painter.circle_stroke(center, radius, Stroke::new(1.5, color));
                        // Zzz indicator
                        painter.text(
                            Pos2::new(center.x + radius, center.y - radius),
                            egui::Align2::LEFT_BOTTOM,
                            "z",
                            egui::FontId::proportional(8.0),
                            Color32::from_rgb(180, 180, 255),
                        );
                    } else {
                        painter.circle_filled(center, radius, color);
                    }

                    // Drive urgency indicator
                    if let Some(drive) = agent.most_urgent_drive {
                        let drive_color = tooltip::drive_color(drive);
                        painter.circle_stroke(center, radius + 1.5, Stroke::new(1.0, drive_color));
                    }

                    // Selection highlight
                    if let EntitySelection::Agent(selected_id) = &selection.current {
                        if *selected_id == agent.id {
                            let pulse = (current_time * 3.0).sin() as f32 * 0.3 + 0.7;
                            painter.circle_stroke(center, radius + 4.0, Stroke::new(2.5 * pulse, Color32::WHITE));
                        }
                    }
                }
            }
        }

        // Handle click selection
        if response.clicked() {
            if let Some(click_pos) = response.interact_pointer_pos() {
                let (tile_x, tile_y) = screen_to_world(click_pos, view_rect, &map_view);

                if tile_x >= 0 && tile_x < world.width as i32 && tile_y >= 0 && tile_y < world.height as i32 {
                    if let Some(new_selection) = find_entity_at(snap, tile_x, tile_y, &map_view) {
                        if selection.current != new_selection {
                            selection.current = new_selection.clone();
                            sim_commands.send(SimulationCommand::SelectEntity(new_selection));
                        }
                    }
                }
            }
        }

        // Show tooltip on hover
        if let Some(pos) = response.hover_pos() {
            let (tile_x, tile_y) = screen_to_world(pos, view_rect, &map_view);

            if tile_x >= 0 && tile_x < world.width as i32 && tile_y >= 0 && tile_y < world.height as i32 {
                response.clone().on_hover_ui_at_pointer(|ui| {
                    ui.set_max_width(280.0);

                    // Terrain header
                    let terrain = world.tiles.iter()
                        .find(|t| t.x == tile_x && t.y == tile_y)
                        .map(|t| t.terrain);
                    tooltip::render_terrain_header(ui, tile_x, tile_y, terrain);

                    // Show all agents at this position
                    let agents_here: Vec<_> = snap.population.agents.iter()
                        .filter(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive)
                        .collect();
                    for agent in &agents_here {
                        ui.separator();
                        tooltip::render_agent_tooltip(ui, agent);
                    }

                    // Show all resources at this position
                    let resources_here: Vec<_> = world.resources.iter()
                        .filter(|r| r.position.x == tile_x && r.position.y == tile_y)
                        .collect();
                    for resource in &resources_here {
                        ui.separator();
                        tooltip::render_resource_tooltip(ui, resource.resource_type, resource.amount, resource.max_amount);
                    }

                    // Show all buildings at this position
                    let buildings_here: Vec<_> = world.buildings.iter()
                        .filter(|b| b.position.x == tile_x && b.position.y == tile_y)
                        .collect();
                    for building in &buildings_here {
                        ui.separator();
                        tooltip::render_building_tooltip(ui, building.building_type, building.completed, building.progress);
                    }
                });
            }
        }

        // Draw minimap
        let minimap_click = if map_view.minimap.enabled {
            draw_minimap(&painter, view_rect, &map_view, snap, &selection, ui)
        } else {
            None
        };

        // Apply minimap click to center view
        if let Some((x, y)) = minimap_click {
            center_on_tile(&mut map_view, x, y, view_rect);
        }

        // Map controls toolbar
        ui.add_space(5.0);
        render_map_controls(ui, &mut map_view, &mut selection, &mut center_request, &mut notifications, snap, current_time);
    });
}

/// Draw grid overlay
fn draw_grid(painter: &egui::Painter, view_rect: Rect, map_view: &MapViewState, world_width: usize, world_height: usize) {
    let size = TILE_SIZE * map_view.zoom;
    let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 25);

    let start_x = ((map_view.offset.0 / size).floor() as i32).max(0);
    let end_x = (((map_view.offset.0 + view_rect.width()) / size).ceil() as i32).min(world_width as i32 + 1);

    for x in start_x..=end_x {
        let screen_x = view_rect.min.x + x as f32 * size - map_view.offset.0;
        if screen_x >= view_rect.min.x && screen_x <= view_rect.max.x {
            painter.line_segment(
                [Pos2::new(screen_x, view_rect.min.y), Pos2::new(screen_x, view_rect.max.y)],
                Stroke::new(1.0, grid_color),
            );
        }
    }

    let start_y = ((map_view.offset.1 / size).floor() as i32).max(0);
    let end_y = (((map_view.offset.1 + view_rect.height()) / size).ceil() as i32).min(world_height as i32 + 1);

    for y in start_y..=end_y {
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
    ui: &mut egui::Ui,
) -> Option<(i32, i32)> {
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

    // Background
    painter.rect_filled(minimap_rect, 4.0, Color32::from_rgba_unmultiplied(0, 0, 0, opacity));
    painter.rect_stroke(minimap_rect, 4.0, Stroke::new(1.0, Color32::from_rgb(100, 100, 100)));

    let scale_x = minimap_size / world.width as f32;
    let scale_y = minimap_size / world.height as f32;
    let scale = scale_x.min(scale_y);

    // Draw terrain
    for tile in &world.tiles {
        let x = minimap_rect.min.x + tile.x as f32 * scale;
        let y = minimap_rect.min.y + tile.y as f32 * scale;
        let tile_size = scale.max(1.0);
        let tile_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(tile_size, tile_size));
        let color = tooltip::terrain_color(tile.terrain);
        painter.rect_filled(tile_rect, 0.0, color);
    }

    // Draw resources
    if map_view.minimap.show_resources {
        for resource in &world.resources {
            let x = minimap_rect.min.x + resource.position.x as f32 * scale + scale / 2.0;
            let y = minimap_rect.min.y + resource.position.y as f32 * scale + scale / 2.0;
            let color = tooltip::resource_color(resource.resource_type);
            painter.circle_filled(Pos2::new(x, y), 1.5, color);

            if let EntitySelection::Resource(pos) = &selection.current {
                if pos.x == resource.position.x && pos.y == resource.position.y {
                    painter.circle_stroke(Pos2::new(x, y), 4.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw buildings
    if map_view.minimap.show_buildings {
        for building in &world.buildings {
            let x = minimap_rect.min.x + building.position.x as f32 * scale;
            let y = minimap_rect.min.y + building.position.y as f32 * scale;
            let bsize = scale.max(2.0);
            let color = tooltip::building_color(building.building_type, building.completed);
            let building_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bsize, bsize));
            painter.rect_filled(building_rect, 0.0, color);

            if let EntitySelection::Building(pos) = &selection.current {
                if pos.x == building.position.x && pos.y == building.position.y {
                    painter.rect_stroke(building_rect.expand(2.0), 0.0, Stroke::new(1.5, Color32::YELLOW));
                }
            }
        }
    }

    // Draw agents
    if map_view.minimap.show_agents {
        for agent in &snapshot.population.agents {
            if !agent.is_alive || !should_show_agent(agent, map_view) {
                continue;
            }

            let x = minimap_rect.min.x + agent.position.0 as f32 * scale + scale / 2.0;
            let y = minimap_rect.min.y + agent.position.1 as f32 * scale + scale / 2.0;
            let color = tooltip::life_stage_map_color(agent.life_stage);
            painter.circle_filled(Pos2::new(x, y), 2.0, color);

            if let EntitySelection::Agent(selected_id) = &selection.current {
                if *selected_id == agent.id {
                    painter.circle_stroke(Pos2::new(x, y), 4.0, Stroke::new(1.5, Color32::WHITE));
                }
            }
        }
    }

    // Draw viewport rectangle
    let vp_x = minimap_rect.min.x + map_view.offset.0 / (TILE_SIZE * map_view.zoom) * scale;
    let vp_y = minimap_rect.min.y + map_view.offset.1 / (TILE_SIZE * map_view.zoom) * scale;
    let vp_w = (view_rect.width() / (TILE_SIZE * map_view.zoom)) * scale;
    let vp_h = (view_rect.height() / (TILE_SIZE * map_view.zoom)) * scale;

    let viewport_rect = Rect::from_min_size(Pos2::new(vp_x, vp_y), Vec2::new(vp_w, vp_h));
    let clamped_viewport = viewport_rect.intersect(minimap_rect);
    if clamped_viewport.width() > 0.0 && clamped_viewport.height() > 0.0 {
        painter.rect_stroke(clamped_viewport, 0.0, Stroke::new(2.0, Color32::WHITE));
    }

    // Minimap labels
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

    // Click/drag on minimap to navigate
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
        ui.label(egui::RichText::new("Zoom:").small());

        if ui.add(egui::Button::new("-").min_size(egui::vec2(24.0, 0.0)))
            .on_hover_text("Zoom out (-)")
            .clicked()
        {
            map_view.zoom = (map_view.zoom - 0.25).max(MIN_ZOOM);
        }

        ui.label(format!("{:.0}%", map_view.zoom * 100.0))
            .on_hover_text("Current zoom level\nScroll wheel to zoom toward cursor");

        if ui.add(egui::Button::new("+").min_size(egui::vec2(24.0, 0.0)))
            .on_hover_text("Zoom in (+/=)")
            .clicked()
        {
            map_view.zoom = (map_view.zoom + 0.25).min(MAX_ZOOM);
        }

        if ui.button("Reset")
            .on_hover_text("Reset view to default zoom and position (Home)")
            .clicked()
        {
            map_view.reset_view();
            notifications.info("View reset", current_time);
        }

        ui.separator();

        // Selection controls
        let has_selection = !matches!(selection.current, EntitySelection::None);

        let center_button = egui::Button::new("Center");
        if ui.add_enabled(has_selection, center_button)
            .on_hover_text(if has_selection {
                "Center view on selected entity (C)"
            } else {
                "Select an entity first"
            })
            .clicked()
        {
            match &selection.current {
                EntitySelection::Agent(id) => {
                    if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == *id && a.is_alive) {
                        center_request.send(CenterMapRequest { x: agent.position.0, y: agent.position.1 });
                        notifications.info("Centered on agent", current_time);
                    }
                }
                EntitySelection::Building(pos) | EntitySelection::Resource(pos) | EntitySelection::Terrain(pos) => {
                    center_request.send(CenterMapRequest { x: pos.x, y: pos.y });
                    notifications.info("Centered on selection", current_time);
                }
                EntitySelection::None => {}
            }
        }

        // Follow mode toggle
        let follow_text = if selection.follow_selected {
            egui::RichText::new("Following").color(egui::Color32::from_rgb(100, 200, 100))
        } else {
            egui::RichText::new("Follow")
        };

        if ui.add_enabled(has_selection, egui::SelectableLabel::new(selection.follow_selected, follow_text))
            .on_hover_text(if has_selection {
                "Auto-center on selected agent as it moves (F)"
            } else {
                "Select an agent first"
            })
            .clicked()
        {
            selection.toggle_follow();
            if selection.follow_selected {
                notifications.info("Follow mode enabled", current_time);
            } else {
                notifications.info("Follow mode disabled", current_time);
            }
        }

        ui.separator();

        // Layer toggles with tooltips
        ui.label(egui::RichText::new("Layers:").small());
        ui.checkbox(&mut map_view.layers.terrain, "T")
            .on_hover_text("Show terrain layer");
        ui.checkbox(&mut map_view.layers.resources, "R")
            .on_hover_text("Show resource deposits");
        ui.checkbox(&mut map_view.layers.buildings, "B")
            .on_hover_text("Show buildings");
        ui.checkbox(&mut map_view.layers.agents, "A")
            .on_hover_text("Show agents");
        ui.checkbox(&mut map_view.layers.grid, "G")
            .on_hover_text("Show grid overlay (G)");

        ui.separator();

        // Agent filter menu
        let filter_color = if map_view.agent_filter.is_filtering() {
            egui::Color32::from_rgb(255, 200, 100)
        } else {
            egui::Color32::GRAY
        };
        let filter_text = egui::RichText::new(if map_view.agent_filter.is_filtering() { "Filter*" } else { "Filter" })
            .color(filter_color);

        ui.menu_button(filter_text, |ui| {
            ui.set_min_width(180.0);
            render_agent_filter_menu(ui, map_view);
        }).response.on_hover_text("Filter which agents are displayed");

        ui.separator();

        // Minimap settings menu
        let minimap_text = if map_view.minimap.enabled {
            egui::RichText::new("Minimap").color(egui::Color32::from_rgb(100, 200, 100))
        } else {
            egui::RichText::new("Minimap").color(egui::Color32::GRAY)
        };

        ui.menu_button(minimap_text, |ui| {
            ui.set_min_width(160.0);
            ui.checkbox(&mut map_view.minimap.enabled, "Show Minimap")
                .on_hover_text("Toggle minimap visibility (M)");
            ui.separator();

            ui.label(egui::RichText::new("Display Layers").small().color(egui::Color32::GRAY));
            ui.checkbox(&mut map_view.minimap.show_resources, "Resources");
            ui.checkbox(&mut map_view.minimap.show_buildings, "Buildings");
            ui.checkbox(&mut map_view.minimap.show_agents, "Agents");
            ui.separator();

            ui.label(egui::RichText::new("Position").small().color(egui::Color32::GRAY));
            ui.horizontal(|ui| {
                let positions = [
                    (MinimapPosition::TopLeft, "TL", "Top-left corner"),
                    (MinimapPosition::TopRight, "TR", "Top-right corner"),
                    (MinimapPosition::BottomLeft, "BL", "Bottom-left corner"),
                    (MinimapPosition::BottomRight, "BR", "Bottom-right corner"),
                ];
                for (pos, label, tooltip) in positions {
                    if ui.selectable_label(map_view.minimap.position == pos, label)
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        map_view.minimap.position = pos;
                    }
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Size:");
                ui.add(egui::Slider::new(&mut map_view.minimap.size, 80.0..=200.0)
                    .suffix("px")
                    .clamping(egui::SliderClamping::Always));
            });
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                ui.add(egui::Slider::new(&mut map_view.minimap.opacity, 0.3..=1.0)
                    .clamping(egui::SliderClamping::Always));
            });
        }).response.on_hover_text("Minimap display settings (M)");

        // Coordinates display on the right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let selection_info = match &selection.current {
                EntitySelection::Agent(id) => {
                    if let Some(agent) = snapshot.population.agents.iter().find(|a| a.id == *id) {
                        format!("Agent @ ({}, {})", agent.position.0, agent.position.1)
                    } else {
                        "Agent (not found)".to_string()
                    }
                }
                EntitySelection::Building(pos) => format!("Building @ ({}, {})", pos.x, pos.y),
                EntitySelection::Resource(pos) => format!("Resource @ ({}, {})", pos.x, pos.y),
                EntitySelection::Terrain(pos) => format!("Terrain @ ({}, {})", pos.x, pos.y),
                EntitySelection::None => "No selection".to_string(),
            };
            ui.label(egui::RichText::new(selection_info).small().color(egui::Color32::GRAY))
                .on_hover_text("Current selection\nTab/Shift+Tab to cycle agents");
        });
    });
}

/// Render agent filter menu
fn render_agent_filter_menu(ui: &mut egui::Ui, map_view: &mut MapViewState) {
    // Quick actions
    ui.horizontal(|ui| {
        if ui.button("Show All")
            .on_hover_text("Show all agents")
            .clicked()
        {
            map_view.agent_filter.reset();
        }
        if ui.button("Adults Only")
            .on_hover_text("Show only adult agents")
            .clicked()
        {
            map_view.agent_filter.show_infant = false;
            map_view.agent_filter.show_child = false;
            map_view.agent_filter.show_adolescent = false;
            map_view.agent_filter.show_adult = true;
            map_view.agent_filter.show_elderly = false;
        }
        if ui.button("Workers")
            .on_hover_text("Show agents with active jobs")
            .clicked()
        {
            map_view.agent_filter.show_sleeping = false;
            map_view.agent_filter.show_idle = false;
        }
    });

    ui.separator();

    // Life stage filter
    ui.label(egui::RichText::new("Life Stage").small().color(Color32::GRAY));
    ui.horizontal(|ui| {
        filter_checkbox(ui, &mut map_view.agent_filter.show_infant, "Infant", "Newborns (0-2 years)");
        filter_checkbox(ui, &mut map_view.agent_filter.show_child, "Child", "Children (2-12 years)");
        filter_checkbox(ui, &mut map_view.agent_filter.show_adolescent, "Teen", "Adolescents (12-18 years)");
    });
    ui.horizontal(|ui| {
        filter_checkbox(ui, &mut map_view.agent_filter.show_adult, "Adult", "Adults (18-60 years)");
        filter_checkbox(ui, &mut map_view.agent_filter.show_elderly, "Elder", "Elderly (60+ years)");
    });

    ui.separator();

    // Gender filter
    ui.label(egui::RichText::new("Gender").small().color(Color32::GRAY));
    ui.horizontal(|ui| {
        filter_checkbox(ui, &mut map_view.agent_filter.show_male, "\u{2642} Male", "Show male agents");
        filter_checkbox(ui, &mut map_view.agent_filter.show_female, "\u{2640} Female", "Show female agents");
    });

    ui.separator();

    // Status filter
    ui.label(egui::RichText::new("Status").small().color(Color32::GRAY));
    ui.horizontal(|ui| {
        filter_checkbox(ui, &mut map_view.agent_filter.show_sleeping, "Sleeping", "Agents currently resting");
        filter_checkbox(ui, &mut map_view.agent_filter.show_idle, "Idle", "Agents without current task");
    });

    ui.separator();

    // Activity filter
    ui.label(egui::RichText::new("Activity").small().color(Color32::GRAY));
    ui.columns(2, |cols| {
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_gathering, "Gathering", "Collecting natural resources");
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_farming, "Farming", "Agricultural work");
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_hunting, "Hunting", "Hunting animals");
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_fishing, "Fishing", "Catching fish");
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_mining, "Mining", "Extracting stone/ore");
        filter_checkbox(&mut cols[0], &mut map_view.agent_filter.show_cooking, "Cooking", "Preparing food");

        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_building, "Building", "Construction work");
        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_crafting, "Crafting", "Creating items");
        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_exploring, "Exploring", "Scouting new areas");
        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_social, "Social", "Social interactions");
        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_caretaking, "Caretaking", "Caring for others");
        filter_checkbox(&mut cols[1], &mut map_view.agent_filter.show_labor, "Labor", "General labor tasks");
    });
}

fn filter_checkbox(ui: &mut egui::Ui, value: &mut bool, label: &str, tooltip: &str) {
    ui.checkbox(value, label).on_hover_text(tooltip);
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

/// Find entity at world position for selection
fn find_entity_at(
    snapshot: &SimulationSnapshot,
    tile_x: i32,
    tile_y: i32,
    map_view: &MapViewState,
) -> Option<EntitySelection> {
    let world = &snapshot.world;

    // Priority: agents > buildings > resources > terrain
    if map_view.layers.agents {
        if let Some(agent) = snapshot.population.agents.iter()
            .find(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive && should_show_agent(a, map_view))
        {
            return Some(EntitySelection::Agent(agent.id));
        }
    }

    if map_view.layers.buildings {
        if let Some(building) = world.buildings.iter()
            .find(|b| b.position.x == tile_x && b.position.y == tile_y)
        {
            return Some(EntitySelection::Building(building.position));
        }
    }

    if map_view.layers.resources {
        if let Some(resource) = world.resources.iter()
            .find(|r| r.position.x == tile_x && r.position.y == tile_y)
        {
            return Some(EntitySelection::Resource(resource.position));
        }
    }

    Some(EntitySelection::Terrain(Position::new(tile_x, tile_y)))
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
