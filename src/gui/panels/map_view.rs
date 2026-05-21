// src/gui/panels/map_view.rs
//! Interactive world map rendering.

use egui::{Ui, Sense, Color32, Rect, Pos2, Vec2, Stroke};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::gui::state::{GuiState, SimulationCommand, EntitySelection};
use crate::world::TerrainType;
use crate::agents::LifeStage;

const TILE_SIZE: f32 = 12.0;

pub fn render_map(
    ui: &mut Ui,
    state: &mut GuiState,
    command_tx: &Sender<SimulationCommand>,
    agent_data_request: &Arc<Mutex<Option<Uuid>>>,
) {
    let Some(snapshot) = &state.latest_snapshot else {
        ui.centered_and_justified(|ui| {
            ui.label("Waiting for simulation data...");
        });
        return;
    };

    // Calculate available space
    let _available_size = ui.available_size();

    // Create a scrollable area for the map
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let world = &snapshot.world;
            let map_width = world.width as f32 * TILE_SIZE * state.map_zoom;
            let map_height = world.height as f32 * TILE_SIZE * state.map_zoom;

            // Allocate space for the map
            let (response, painter) = ui.allocate_painter(
                Vec2::new(map_width, map_height),
                Sense::click_and_drag(),
            );

            let rect = response.rect;

            // Handle zoom with scroll wheel
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 && response.hovered() {
                let zoom_factor = 1.0 + scroll_delta * 0.001;
                state.map_zoom = (state.map_zoom * zoom_factor).clamp(0.5, 4.0);
            }

            // Draw terrain tiles
            for tile in &world.tiles {
                let x = rect.min.x + tile.x as f32 * TILE_SIZE * state.map_zoom;
                let y = rect.min.y + tile.y as f32 * TILE_SIZE * state.map_zoom;
                let size = TILE_SIZE * state.map_zoom;

                let tile_rect = Rect::from_min_size(
                    Pos2::new(x, y),
                    Vec2::new(size, size),
                );

                let color = terrain_color(tile.terrain);
                painter.rect_filled(tile_rect, 0.0, color);
            }

            // Draw resources
            for resource in &world.resources {
                let x = rect.min.x + resource.position.x as f32 * TILE_SIZE * state.map_zoom;
                let y = rect.min.y + resource.position.y as f32 * TILE_SIZE * state.map_zoom;
                let size = TILE_SIZE * state.map_zoom;

                let center = Pos2::new(x + size / 2.0, y + size / 2.0);
                let radius = size * 0.3;

                let color = resource_color(resource.resource_type);
                painter.circle_filled(center, radius, color);
            }

            // Draw buildings
            for building in &world.buildings {
                let x = rect.min.x + building.position.x as f32 * TILE_SIZE * state.map_zoom;
                let y = rect.min.y + building.position.y as f32 * TILE_SIZE * state.map_zoom;
                let size = TILE_SIZE * state.map_zoom;

                let building_rect = Rect::from_min_size(
                    Pos2::new(x + size * 0.1, y + size * 0.1),
                    Vec2::new(size * 0.8, size * 0.8),
                );

                let color = if building.completed {
                    Color32::from_rgb(139, 90, 43) // Brown
                } else {
                    Color32::from_rgb(180, 150, 100) // Light brown (under construction)
                };

                painter.rect_filled(building_rect, 2.0, color);
            }

            // Draw agents
            for agent in &snapshot.population.agents {
                if !agent.is_alive {
                    continue;
                }

                let x = rect.min.x + agent.position.0 as f32 * TILE_SIZE * state.map_zoom;
                let y = rect.min.y + agent.position.1 as f32 * TILE_SIZE * state.map_zoom;
                let size = TILE_SIZE * state.map_zoom;

                let center = Pos2::new(x + size / 2.0, y + size / 2.0);
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

                // Selection highlight
                if let EntitySelection::Agent(selected_id) = &state.selected {
                    if *selected_id == agent.id {
                        painter.circle_stroke(center, radius + 3.0, Stroke::new(2.0, Color32::WHITE));
                    }
                }
            }

            // Handle clicks for selection
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let tile_x = ((pos.x - rect.min.x) / (TILE_SIZE * state.map_zoom)) as i32;
                    let tile_y = ((pos.y - rect.min.y) / (TILE_SIZE * state.map_zoom)) as i32;

                    // Check if we clicked on an agent
                    let clicked_agent = snapshot.population.agents.iter()
                        .find(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive);

                    if let Some(agent) = clicked_agent {
                        state.selected = EntitySelection::Agent(agent.id);
                        let _ = command_tx.send(SimulationCommand::SelectEntity(EntitySelection::Agent(agent.id)));

                        // Request detailed agent data
                        if let Ok(mut request) = agent_data_request.lock() {
                            *request = Some(agent.id);
                        }
                    } else {
                        // Check for building at position
                        let clicked_building = world.buildings.iter()
                            .find(|b| b.position.x == tile_x && b.position.y == tile_y);

                        if let Some(building) = clicked_building {
                            state.selected = EntitySelection::Building(building.position);
                        } else {
                            // Check for resource
                            let clicked_resource = world.resources.iter()
                                .find(|r| r.position.x == tile_x && r.position.y == tile_y);

                            if let Some(resource) = clicked_resource {
                                state.selected = EntitySelection::Resource(resource.position);
                            } else {
                                // Select terrain
                                state.selected = EntitySelection::Terrain(crate::world::Position::new(tile_x, tile_y));
                            }
                        }
                    }
                }
            }

            // Show coordinates on hover using on_hover_ui_at_pointer
            if let Some(pos) = response.hover_pos() {
                let tile_x = ((pos.x - rect.min.x) / (TILE_SIZE * state.map_zoom)) as i32;
                let tile_y = ((pos.y - rect.min.y) / (TILE_SIZE * state.map_zoom)) as i32;

                response.clone().on_hover_ui_at_pointer(|ui| {
                    ui.label(format!("Position: ({}, {})", tile_x, tile_y));

                    // Show what's at this position
                    if let Some(agent) = snapshot.population.agents.iter()
                        .find(|a| a.position.0 == tile_x && a.position.1 == tile_y && a.is_alive)
                    {
                        ui.label(format!("Agent: {:?}", agent.life_stage));
                        ui.label(format!("Health: {:.0}%", agent.health));
                    }
                });
            }
        });

    // Zoom controls
    ui.horizontal(|ui| {
        if ui.button("-").clicked() {
            state.map_zoom = (state.map_zoom - 0.25).max(0.5);
        }
        ui.label(format!("Zoom: {:.0}%", state.map_zoom * 100.0));
        if ui.button("+").clicked() {
            state.map_zoom = (state.map_zoom + 0.25).min(4.0);
        }
        if ui.button("Reset").clicked() {
            state.map_zoom = 1.0;
        }
    });
}

fn terrain_color(terrain: TerrainType) -> Color32 {
    match terrain {
        TerrainType::Plains => Color32::from_rgb(144, 238, 144),   // Light green
        TerrainType::Meadow => Color32::from_rgb(124, 252, 0),     // Lawn green
        TerrainType::Forest => Color32::from_rgb(34, 139, 34),     // Forest green
        TerrainType::Hills => Color32::from_rgb(139, 137, 112),    // Khaki
        TerrainType::Mountain => Color32::from_rgb(128, 128, 128), // Gray
        TerrainType::Water => Color32::from_rgb(65, 105, 225),     // Royal blue
        TerrainType::Desert => Color32::from_rgb(238, 203, 173),   // Peach puff
        TerrainType::Wetland => Color32::from_rgb(85, 107, 47),    // Dark olive green
        TerrainType::Beach => Color32::from_rgb(238, 214, 175),    // Sandy
        TerrainType::Riverbank => Color32::from_rgb(107, 142, 35), // Olive drab
    }
}

fn resource_color(resource_type: crate::world::ResourceType) -> Color32 {
    use crate::world::ResourceType;
    match resource_type {
        ResourceType::Wood => Color32::from_rgb(139, 69, 19),      // Saddle brown
        ResourceType::Stone => Color32::from_rgb(169, 169, 169),   // Dark gray
        ResourceType::Iron => Color32::from_rgb(112, 128, 144),    // Slate gray
        ResourceType::Food => Color32::from_rgb(255, 99, 71),      // Tomato
        ResourceType::Water => Color32::from_rgb(0, 191, 255),     // Deep sky blue
        ResourceType::Coal => Color32::from_rgb(47, 79, 79),       // Dark slate gray
        ResourceType::Grain => Color32::from_rgb(255, 215, 0),     // Gold
        ResourceType::Herbs => Color32::from_rgb(0, 128, 0),       // Green
        _ => Color32::from_rgb(200, 200, 200),                     // Light gray (default)
    }
}

fn life_stage_color(life_stage: LifeStage) -> Color32 {
    match life_stage {
        LifeStage::Infant => Color32::from_rgb(255, 182, 193),     // Light pink
        LifeStage::Child => Color32::from_rgb(135, 206, 250),      // Light sky blue
        LifeStage::Adolescent => Color32::from_rgb(144, 238, 144), // Light green
        LifeStage::Adult => Color32::from_rgb(255, 255, 255),      // White
        LifeStage::Elderly => Color32::from_rgb(192, 192, 192),    // Silver
    }
}
