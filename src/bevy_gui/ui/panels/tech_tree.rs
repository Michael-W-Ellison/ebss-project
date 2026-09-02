// src/bevy_gui/ui/panels/tech_tree.rs
//! Visual technology tree panel showing progression through eras.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui::{Color32, Pos2, Vec2, Rect, Stroke, RichText, Sense};

use crate::bevy_gui::resources::{PanelVisibility, TechTreeData};
use crate::gui::state::{TechTreeSnapshot, TechNodeData, TechStatus};

const NODE_WIDTH: f32 = 140.0;
const NODE_HEIGHT: f32 = 60.0;
const NODE_SPACING_X: f32 = 180.0;
const NODE_SPACING_Y: f32 = 90.0;
const ERA_HEADER_HEIGHT: f32 = 30.0;

struct EraColumn {
    name: &'static str,
    color: Color32,
    header_color: Color32,
}

const ERAS: &[EraColumn] = &[
    EraColumn {
        name: "Stone Age",
        color: Color32::from_rgb(139, 119, 101),
        header_color: Color32::from_rgb(101, 67, 33),
    },
    EraColumn {
        name: "Copper Age",
        color: Color32::from_rgb(184, 115, 51),
        header_color: Color32::from_rgb(140, 90, 40),
    },
    EraColumn {
        name: "Bronze Age",
        color: Color32::from_rgb(205, 127, 50),
        header_color: Color32::from_rgb(160, 100, 40),
    },
    EraColumn {
        name: "Iron Age",
        color: Color32::from_rgb(112, 128, 144),
        header_color: Color32::from_rgb(70, 80, 90),
    },
    EraColumn {
        name: "Medieval",
        color: Color32::from_rgb(128, 0, 128),
        header_color: Color32::from_rgb(75, 0, 130),
    },
];

pub fn render_tech_tree_panel(
    mut egui_ctx: EguiContexts,
    mut panels: ResMut<PanelVisibility>,
    mut tech_data: ResMut<TechTreeData>,
) {
    if !panels.tech_tree {
        return;
    }

    let mut close_requested = false;

    egui::Window::new("Technology Tree")
        .default_size([900.0, 600.0])
        .resizable(true)
        .collapsible(false)
        .show(egui_ctx.ctx_mut(), |ui| {
            // Header with close button
            ui.horizontal(|ui| {
                ui.heading("Technology Tree");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
            ui.separator();

            let tech_snapshot = match &tech_data.snapshot {
                Some(snapshot) => snapshot.clone(),
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label("Loading technology data...");
                        ui.spinner();
                    });
                    return;
                }
            };

            render_progress_summary(ui, &tech_snapshot);
            ui.separator();

            let available_height = ui.available_height();

            ui.horizontal(|ui| {
                let tree_width = ui.available_width() - 220.0;

                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 20, 30))
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(tree_width, available_height - 10.0));
                        render_tech_tree_visual(ui, &mut tech_data, &tech_snapshot);
                    });

                ui.separator();

                ui.vertical(|ui| {
                    ui.set_min_width(200.0);
                    render_tech_details(ui, &tech_data, &tech_snapshot);
                });
            });
        });

    if close_requested {
        panels.tech_tree = false;
    }
}

fn render_progress_summary(ui: &mut egui::Ui, snapshot: &TechTreeSnapshot) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Current Era").small());
            ui.label(RichText::new(&snapshot.current_era).size(16.0).strong());
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.label(RichText::new("Discovered").small());
            ui.label(RichText::new(format!("{} / {}",
                snapshot.total_discovered,
                snapshot.total_technologies
            )).size(16.0));
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.label(RichText::new("Progress").small());
            let progress = snapshot.total_discovered as f32 / snapshot.total_technologies.max(1) as f32;
            ui.add(egui::ProgressBar::new(progress)
                .desired_width(150.0)
                .text(format!("{:.0}%", progress * 100.0)));
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.label(RichText::new("By Era").small());
            ui.horizontal(|ui| {
                for (i, era) in ERAS.iter().enumerate() {
                    let count = snapshot.nodes.iter()
                        .filter(|n| n.era_index == i && n.status == TechStatus::Discovered)
                        .count();
                    let total = snapshot.nodes.iter()
                        .filter(|n| n.era_index == i)
                        .count();
                    if total > 0 {
                        ui.colored_label(era.color, format!("{}/{}", count, total));
                    }
                }
            });
        });
    });
}

fn render_tech_tree_visual(
    ui: &mut egui::Ui,
    tech_data: &mut ResMut<TechTreeData>,
    snapshot: &TechTreeSnapshot,
) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut era_tech_counts: Vec<usize> = vec![0; ERAS.len()];
            for node in &snapshot.nodes {
                if node.era_index < ERAS.len() {
                    era_tech_counts[node.era_index] += 1;
                }
            }

            let max_techs_in_era = *era_tech_counts.iter().max().unwrap_or(&1);
            let tree_height = ERA_HEADER_HEIGHT + (max_techs_in_era as f32 * NODE_SPACING_Y) + 50.0;
            let tree_width = (ERAS.len() as f32 * NODE_SPACING_X) + 50.0;

            let (response, painter) = ui.allocate_painter(
                Vec2::new(tree_width, tree_height),
                Sense::click(),
            );
            let rect = response.rect;

            // Draw era columns
            for (i, era) in ERAS.iter().enumerate() {
                let x = rect.min.x + 20.0 + (i as f32 * NODE_SPACING_X);
                let column_rect = Rect::from_min_size(
                    Pos2::new(x, rect.min.y),
                    Vec2::new(NODE_SPACING_X - 10.0, tree_height),
                );

                painter.rect_filled(
                    column_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(
                        era.color.r(),
                        era.color.g(),
                        era.color.b(),
                        20,
                    ),
                );

                let header_rect = Rect::from_min_size(
                    Pos2::new(x, rect.min.y),
                    Vec2::new(NODE_SPACING_X - 10.0, ERA_HEADER_HEIGHT),
                );
                painter.rect_filled(header_rect, 4.0, era.header_color);
                painter.text(
                    header_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    era.name,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }

            // Position nodes
            let mut era_y_offsets: Vec<f32> = vec![ERA_HEADER_HEIGHT + 20.0; ERAS.len()];

            let mut sorted_nodes: Vec<&TechNodeData> = snapshot.nodes.iter().collect();
            sorted_nodes.sort_by(|a, b| {
                a.era_index.cmp(&b.era_index)
                    .then_with(|| a.prerequisites.len().cmp(&b.prerequisites.len()))
            });

            let mut node_positions: std::collections::BTreeMap<String, Pos2> = std::collections::BTreeMap::new();

            for node in &sorted_nodes {
                if node.era_index >= ERAS.len() {
                    continue;
                }

                let x = rect.min.x + 20.0 + (node.era_index as f32 * NODE_SPACING_X) + (NODE_SPACING_X - NODE_WIDTH) / 2.0;
                let y = rect.min.y + era_y_offsets[node.era_index];

                node_positions.insert(node.id.clone(), Pos2::new(x + NODE_WIDTH / 2.0, y + NODE_HEIGHT / 2.0));
                era_y_offsets[node.era_index] += NODE_SPACING_Y;
            }

            // Draw connections
            for node in &snapshot.nodes {
                if let Some(&node_pos) = node_positions.get(&node.id) {
                    for prereq_id in &node.prerequisites {
                        if let Some(&prereq_pos) = node_positions.get(prereq_id) {
                            let line_color = if node.status == TechStatus::Discovered {
                                Color32::from_rgb(100, 200, 100)
                            } else if node.status == TechStatus::Discoverable {
                                Color32::from_rgb(200, 200, 100)
                            } else {
                                Color32::from_rgb(80, 80, 80)
                            };

                            let start = Pos2::new(prereq_pos.x + NODE_WIDTH / 2.0 - 10.0, prereq_pos.y);
                            let end = Pos2::new(node_pos.x - NODE_WIDTH / 2.0 + 10.0, node_pos.y);

                            let mid_x = (start.x + end.x) / 2.0;
                            let ctrl1 = Pos2::new(mid_x, start.y);
                            let ctrl2 = Pos2::new(mid_x, end.y);

                            painter.line_segment([start, ctrl1], Stroke::new(2.0, line_color));
                            painter.line_segment([ctrl1, ctrl2], Stroke::new(2.0, line_color));
                            painter.line_segment([ctrl2, end], Stroke::new(2.0, line_color));

                            let arrow_size = 6.0;
                            let arrow_p1 = Pos2::new(end.x - arrow_size, end.y - arrow_size / 2.0);
                            let arrow_p2 = Pos2::new(end.x - arrow_size, end.y + arrow_size / 2.0);
                            painter.add(egui::Shape::convex_polygon(
                                vec![end, arrow_p1, arrow_p2],
                                line_color,
                                Stroke::NONE,
                            ));
                        }
                    }
                }
            }

            // Draw nodes
            for node in &snapshot.nodes {
                if let Some(&center) = node_positions.get(&node.id) {
                    let node_rect = Rect::from_center_size(center, Vec2::new(NODE_WIDTH, NODE_HEIGHT));
                    draw_tech_node(&painter, node_rect, node, tech_data.selected_tech.as_ref());
                }
            }

            // Handle click
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let mut clicked_node = None;
                    for node in &snapshot.nodes {
                        if let Some(&center) = node_positions.get(&node.id) {
                            let node_rect = Rect::from_center_size(center, Vec2::new(NODE_WIDTH, NODE_HEIGHT));
                            if node_rect.contains(pos) {
                                clicked_node = Some(node.id.clone());
                                break;
                            }
                        }
                    }
                    tech_data.selected_tech = clicked_node;
                }
            }
        });
}

fn draw_tech_node(
    painter: &egui::Painter,
    rect: Rect,
    node: &TechNodeData,
    selected_tech: Option<&String>,
) {
    let (bg_color, border_color, text_color) = match node.status {
        TechStatus::Discovered => (
            Color32::from_rgb(40, 80, 40),
            Color32::from_rgb(100, 200, 100),
            Color32::WHITE,
        ),
        TechStatus::InProgress => (
            Color32::from_rgb(80, 70, 30),
            Color32::from_rgb(200, 180, 50),
            Color32::WHITE,
        ),
        TechStatus::Discoverable => (
            Color32::from_rgb(50, 50, 70),
            Color32::from_rgb(100, 100, 150),
            Color32::from_rgb(200, 200, 220),
        ),
        TechStatus::Unknown => (
            Color32::from_rgb(30, 30, 35),
            Color32::from_rgb(60, 60, 70),
            Color32::from_rgb(100, 100, 110),
        ),
    };

    let is_selected = selected_tech == Some(&node.id);
    let actual_border = if is_selected { Color32::WHITE } else { border_color };
    let border_width = if is_selected { 3.0 } else { 2.0 };

    painter.rect_filled(rect, 6.0, bg_color);
    painter.rect_stroke(rect, 6.0, Stroke::new(border_width, actual_border));

    // Status icon
    let icon_pos = Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0);
    match node.status {
        TechStatus::Discovered => {
            painter.circle_filled(icon_pos, 6.0, Color32::from_rgb(100, 200, 100));
            painter.text(
                icon_pos,
                egui::Align2::CENTER_CENTER,
                "✓",
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
        TechStatus::InProgress => {
            painter.circle_stroke(icon_pos, 6.0, Stroke::new(2.0, Color32::from_rgb(200, 180, 50)));
        }
        TechStatus::Discoverable => {
            painter.circle_stroke(icon_pos, 6.0, Stroke::new(2.0, Color32::from_rgb(100, 100, 150)));
        }
        TechStatus::Unknown => {
            painter.circle_filled(icon_pos, 6.0, Color32::from_rgb(50, 50, 60));
            painter.text(
                icon_pos,
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(80, 80, 90),
            );
        }
    }

    // Tech name
    let name = if node.name.len() > 18 {
        format!("{}...", &node.name[..15])
    } else {
        node.name.clone()
    };

    painter.text(
        Pos2::new(rect.center().x, rect.min.y + 25.0),
        egui::Align2::CENTER_CENTER,
        &name,
        egui::FontId::proportional(12.0),
        text_color,
    );

    // Progress bar for in-progress techs
    if node.status == TechStatus::InProgress && node.discovery_progress > 0 {
        let bar_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + 10.0, rect.max.y - 15.0),
            Vec2::new(rect.width() - 20.0, 8.0),
        );
        painter.rect_filled(bar_rect, 2.0, Color32::from_rgb(40, 40, 50));

        let progress = node.discovery_progress as f32 / 100.0;
        let fill_rect = Rect::from_min_size(
            bar_rect.min,
            Vec2::new(bar_rect.width() * progress, bar_rect.height()),
        );
        painter.rect_filled(fill_rect, 2.0, Color32::from_rgb(200, 180, 50));
    }

    // Agent count for discovered techs
    if node.status == TechStatus::Discovered && node.agents_with_knowledge > 0 {
        let count_pos = Pos2::new(rect.max.x - 15.0, rect.max.y - 12.0);
        painter.text(
            count_pos,
            egui::Align2::CENTER_CENTER,
            format!("{}", node.agents_with_knowledge),
            egui::FontId::proportional(10.0),
            Color32::from_rgb(150, 150, 150),
        );
    }
}

fn render_tech_details(
    ui: &mut egui::Ui,
    tech_data: &TechTreeData,
    snapshot: &TechTreeSnapshot,
) {
    ui.heading("Details");
    ui.separator();

    let Some(selected_id) = &tech_data.selected_tech else {
        ui.label("Click a technology to view details.");
        ui.add_space(10.0);

        if !snapshot.discovery_history.is_empty() {
            ui.heading("Recent Discoveries");
            for (tick, tech_id) in snapshot.discovery_history.iter().rev().take(5) {
                if let Some(node) = snapshot.nodes.iter().find(|n| n.id == *tech_id) {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::from_rgb(100, 200, 100), "✓");
                        ui.label(&node.name);
                        ui.label(RichText::new(format!("(tick {})", tick)).small().color(Color32::GRAY));
                    });
                }
            }
        }
        return;
    };

    let Some(node) = snapshot.nodes.iter().find(|n| n.id == *selected_id) else {
        ui.label("Technology not found.");
        return;
    };

    ui.label(RichText::new(&node.name).size(16.0).strong());
    ui.label(RichText::new(&node.era).small().color(era_color(node.era_index)));
    ui.add_space(5.0);

    let (status_text, status_color) = match node.status {
        TechStatus::Discovered => ("Discovered", Color32::from_rgb(100, 200, 100)),
        TechStatus::InProgress => ("In Progress", Color32::from_rgb(200, 180, 50)),
        TechStatus::Discoverable => ("Discoverable", Color32::from_rgb(100, 100, 150)),
        TechStatus::Unknown => ("Unknown", Color32::GRAY),
    };
    ui.colored_label(status_color, status_text);

    ui.add_space(10.0);

    ui.label(RichText::new("Description").strong());
    ui.label(&node.description);

    ui.add_space(10.0);

    if !node.prerequisites.is_empty() {
        ui.label(RichText::new("Prerequisites").strong());
        for prereq_id in &node.prerequisites {
            if let Some(prereq) = snapshot.nodes.iter().find(|n| n.id == *prereq_id) {
                let color = if prereq.status == TechStatus::Discovered {
                    Color32::from_rgb(100, 200, 100)
                } else {
                    Color32::from_rgb(200, 100, 100)
                };
                ui.horizontal(|ui| {
                    ui.colored_label(color, if prereq.status == TechStatus::Discovered { "✓" } else { "✗" });
                    ui.label(&prereq.name);
                });
            }
        }
        ui.add_space(5.0);
    }

    if !node.unlocks.is_empty() {
        ui.label(RichText::new("Unlocks").strong());
        for unlock in &node.unlocks {
            ui.label(format!("• {}", unlock));
        }
        ui.add_space(5.0);
    }

    if node.status == TechStatus::InProgress {
        ui.label(RichText::new("Progress").strong());
        ui.add(egui::ProgressBar::new(node.discovery_progress as f32 / 100.0)
            .text(format!("{}%", node.discovery_progress)));
        ui.add_space(5.0);
    }

    if node.status == TechStatus::Discovered {
        ui.label(RichText::new("Knowledge Spread").strong());
        ui.label(format!("{} agents know this technology", node.agents_with_knowledge));

        if let Some(tick) = node.discovery_tick {
            ui.label(RichText::new(format!("First discovered at tick {}", tick)).small());
        }
    }
}

fn era_color(era_index: usize) -> Color32 {
    if era_index < ERAS.len() {
        ERAS[era_index].color
    } else {
        Color32::GRAY
    }
}
