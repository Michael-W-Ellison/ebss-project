// src/gui/panels/relationship_graph.rs
//! Relationship graph panel showing agent social network visualization.

use egui::{Ui, Color32, RichText, Pos2, Rect, Stroke};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agents::LifeStage;
use crate::gui::state::{
    GuiState, EntitySelection, RelationshipGraphSnapshot, RelationshipGraphNode,
    RelationshipEdge, RelationshipFilter, GraphLayoutMode, GraphNodePosition,
};

const NODE_RADIUS: f32 = 12.0;
const MIN_EDGE_THICKNESS: f32 = 0.5;
const MAX_EDGE_THICKNESS: f32 = 4.0;

/// Render the relationship graph panel
pub fn render_relationship_graph(ui: &mut Ui, state: &mut GuiState) {
    ui.heading("Relationship Graph");
    ui.separator();

    // Control bar
    render_controls(ui, state);
    ui.separator();

    // Main graph area
    let available_rect = ui.available_rect_before_wrap();
    render_graph_area(ui, state, available_rect);

    // Status bar
    ui.separator();
    render_status_bar(ui, state);
}

/// Render the control bar with filters and layout options
fn render_controls(ui: &mut Ui, state: &mut GuiState) {
    ui.horizontal(|ui| {
        // Layout mode selector
        ui.label("Layout:");
        egui::ComboBox::from_id_salt("layout_mode")
            .selected_text(match state.relationship_graph_state.layout_mode {
                GraphLayoutMode::ForceDirected => "Force Directed",
                GraphLayoutMode::Circular => "Circular",
                GraphLayoutMode::Spatial => "World Position",
            })
            .show_ui(ui, |ui| {
                if ui.selectable_value(
                    &mut state.relationship_graph_state.layout_mode,
                    GraphLayoutMode::ForceDirected,
                    "Force Directed"
                ).clicked() {
                    state.relationship_graph_state.needs_layout = true;
                }
                if ui.selectable_value(
                    &mut state.relationship_graph_state.layout_mode,
                    GraphLayoutMode::Circular,
                    "Circular"
                ).clicked() {
                    state.relationship_graph_state.needs_layout = true;
                }
                if ui.selectable_value(
                    &mut state.relationship_graph_state.layout_mode,
                    GraphLayoutMode::Spatial,
                    "World Position"
                ).clicked() {
                    state.relationship_graph_state.needs_layout = true;
                }
            });

        ui.separator();

        // Zoom controls
        ui.label("Zoom:");
        if ui.button("-").clicked() {
            state.relationship_graph_state.zoom = (state.relationship_graph_state.zoom * 0.8).max(0.2);
        }
        ui.label(format!("{:.0}%", state.relationship_graph_state.zoom * 100.0));
        if ui.button("+").clicked() {
            state.relationship_graph_state.zoom = (state.relationship_graph_state.zoom * 1.25).min(5.0);
        }
        if ui.button("Reset").clicked() {
            state.relationship_graph_state.zoom = 1.0;
            state.relationship_graph_state.offset = (0.0, 0.0);
        }

        ui.separator();

        // Labels toggle
        ui.checkbox(&mut state.relationship_graph_state.show_labels, "Labels");

        ui.separator();

        // Filter dropdown
        ui.menu_button("Filters", |ui| {
            render_filter_menu(ui, &mut state.relationship_graph_state.filter);
        });
    });

    // Focus on selected agent
    if let Some(snapshot) = &state.relationship_graph_snapshot {
        if snapshot.nodes.len() > 10 {
            ui.horizontal(|ui| {
                ui.label("Focus:");
                egui::ComboBox::from_id_salt("focus_agent")
                    .selected_text(
                        state.relationship_graph_state.focus_agent
                            .map(|id| format!("{:.8}...", id))
                            .unwrap_or_else(|| "All Agents".to_string())
                    )
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(
                            state.relationship_graph_state.focus_agent.is_none(),
                            "All Agents"
                        ).clicked() {
                            state.relationship_graph_state.focus_agent = None;
                            state.relationship_graph_state.needs_layout = true;
                        }
                        for node in &snapshot.nodes {
                            if ui.selectable_label(
                                state.relationship_graph_state.focus_agent == Some(node.agent_id),
                                format!("{:.8}...", node.agent_id)
                            ).clicked() {
                                state.relationship_graph_state.focus_agent = Some(node.agent_id);
                                state.relationship_graph_state.needs_layout = true;
                            }
                        }
                    });
            });
        }
    }
}

/// Render the filter menu
fn render_filter_menu(ui: &mut Ui, filter: &mut RelationshipFilter) {
    ui.label(RichText::new("Relationship Types").strong());
    ui.checkbox(&mut filter.show_parent, "Parent");
    ui.checkbox(&mut filter.show_child, "Child");
    ui.checkbox(&mut filter.show_sibling, "Sibling");
    ui.checkbox(&mut filter.show_partner, "Partner");
    ui.checkbox(&mut filter.show_friend, "Friend");
    ui.checkbox(&mut filter.show_acquaintance, "Acquaintance");
    ui.checkbox(&mut filter.show_rival, "Rival");
    ui.checkbox(&mut filter.show_enemy, "Enemy");

    ui.separator();
    ui.label(RichText::new("Bond Strength").strong());
    ui.add(egui::Slider::new(&mut filter.min_bond_strength, -1.0..=1.0)
        .text("Min"));
}

/// Render the main graph visualization area
fn render_graph_area(ui: &mut Ui, state: &mut GuiState, rect: Rect) {
    let snapshot = match &state.relationship_graph_snapshot {
        Some(s) => s.clone(),
        None => {
            ui.centered_and_justified(|ui| {
                ui.label("Loading relationship data...");
            });
            return;
        }
    };

    if snapshot.nodes.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No agents to display.");
        });
        return;
    }

    // Compute layout if needed
    if state.relationship_graph_state.needs_layout {
        compute_layout(state, &snapshot, rect);
        state.relationship_graph_state.needs_layout = false;
    }

    // Run a few iterations of force-directed layout if using that mode
    if state.relationship_graph_state.layout_mode == GraphLayoutMode::ForceDirected {
        if state.relationship_graph_state.layout_iterations < 100 {
            run_force_directed_iteration(state, &snapshot);
            state.relationship_graph_state.layout_iterations += 1;
            ui.ctx().request_repaint();
        }
    }

    // Get painter for custom drawing
    let (response, painter) = ui.allocate_painter(rect.size(), egui::Sense::click_and_drag());
    let graph_rect = response.rect;

    // Handle pan with drag
    if response.dragged() {
        let delta = response.drag_delta();
        state.relationship_graph_state.offset.0 += delta.x;
        state.relationship_graph_state.offset.1 += delta.y;
    }

    // Handle zoom with scroll
    let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
    if scroll_delta != 0.0 && graph_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default())) {
        let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
        state.relationship_graph_state.zoom = (state.relationship_graph_state.zoom * zoom_factor).clamp(0.2, 5.0);
    }

    // Background
    painter.rect_filled(graph_rect, 0.0, Color32::from_rgb(25, 25, 35));

    let center = graph_rect.center();
    let zoom = state.relationship_graph_state.zoom;
    let offset = state.relationship_graph_state.offset;

    // Build position lookup
    let positions: HashMap<Uuid, Pos2> = state.relationship_graph_state.node_positions.iter()
        .map(|(id, pos)| {
            let screen_x = center.x + (pos.x + offset.0) * zoom;
            let screen_y = center.y + (pos.y + offset.1) * zoom;
            (*id, Pos2::new(screen_x, screen_y))
        })
        .collect();

    // Draw edges first (behind nodes)
    for node in &snapshot.nodes {
        if let Some(&from_pos) = positions.get(&node.agent_id) {
            for edge in &node.relationships {
                if !should_show_edge(edge, &state.relationship_graph_state.filter) {
                    continue;
                }

                if let Some(&to_pos) = positions.get(&edge.target_id) {
                    let color = relationship_type_color(&edge.relationship_type);
                    let thickness = edge_thickness(edge.bond_strength);
                    let alpha = ((edge.bond_strength.abs() * 0.5 + 0.5) * 255.0) as u8;
                    let stroke_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

                    painter.line_segment([from_pos, to_pos], Stroke::new(thickness * zoom, stroke_color));
                }
            }
        }
    }

    // Draw nodes
    let mut clicked_agent: Option<Uuid> = None;
    let mut hovered_agent: Option<Uuid> = None;

    for node in &snapshot.nodes {
        if let Some(&pos) = positions.get(&node.agent_id) {
            if !graph_rect.contains(pos) {
                continue;
            }

            let node_radius = NODE_RADIUS * zoom;
            let node_color = life_stage_color(node.life_stage);

            // Check hover/click
            let mouse_pos = ui.input(|i| i.pointer.hover_pos());
            let is_hovered = mouse_pos.map(|mp| (mp - pos).length() < node_radius).unwrap_or(false);
            let is_selected = state.relationship_graph_state.selected_agent == Some(node.agent_id);

            if is_hovered {
                hovered_agent = Some(node.agent_id);
            }

            // Draw selection ring
            if is_selected {
                painter.circle_stroke(pos, node_radius + 4.0, Stroke::new(2.0, Color32::WHITE));
            }

            // Draw hover ring
            if is_hovered {
                painter.circle_stroke(pos, node_radius + 2.0, Stroke::new(1.5, Color32::YELLOW));
            }

            // Draw node
            painter.circle_filled(pos, node_radius, node_color);

            // Health indicator (border)
            let health_color = health_to_color(node.health);
            painter.circle_stroke(pos, node_radius, Stroke::new(2.0, health_color));

            // Draw label
            if state.relationship_graph_state.show_labels && zoom > 0.5 {
                let label = format!("{:.4}", node.agent_id);
                painter.text(
                    Pos2::new(pos.x, pos.y + node_radius + 8.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    egui::FontId::proportional(10.0 * zoom.min(1.5)),
                    Color32::LIGHT_GRAY,
                );
            }

            // Check for click
            if is_hovered && response.clicked() {
                clicked_agent = Some(node.agent_id);
            }
        }
    }

    // Update state
    state.relationship_graph_state.hovered_agent = hovered_agent;
    if let Some(agent_id) = clicked_agent {
        state.relationship_graph_state.selected_agent = Some(agent_id);
        state.selected = EntitySelection::Agent(agent_id);
    }

    // Show tooltip for hovered agent
    if let Some(hovered_id) = hovered_agent {
        if let Some(node) = snapshot.nodes.iter().find(|n| n.agent_id == hovered_id) {
            egui::show_tooltip_at_pointer(
                ui.ctx(),
                egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("agent_tooltip_layer")),
                egui::Id::new("agent_tooltip"),
                |ui| {
                    render_agent_tooltip(ui, node);
                },
            );
        }
    }
}

/// Render tooltip for an agent node
fn render_agent_tooltip(ui: &mut Ui, node: &RelationshipGraphNode) {
    ui.label(RichText::new(format!("Agent: {:.8}...", node.agent_id)).strong());
    ui.label(format!("Life Stage: {:?}", node.life_stage));
    ui.label(format!("Health: {:.0}%", node.health));
    ui.label(format!("Position: ({}, {})", node.position.0, node.position.1));
    ui.separator();
    ui.label(format!("Relationships: {}", node.relationships.len()));

    if !node.relationships.is_empty() {
        ui.label(RichText::new("Top relationships:").small());
        for rel in node.relationships.iter().take(5) {
            let color = relationship_type_color(&rel.relationship_type);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&rel.relationship_type).color(color));
                ui.label(format!("({:+.2})", rel.bond_strength));
            });
        }
    }
}

/// Render the status bar
fn render_status_bar(ui: &mut Ui, state: &GuiState) {
    ui.horizontal(|ui| {
        if let Some(snapshot) = &state.relationship_graph_snapshot {
            ui.label(format!("Agents: {}", snapshot.nodes.len()));

            let total_relationships: usize = snapshot.nodes.iter()
                .map(|n| n.relationships.len())
                .sum();
            ui.label(format!("| Relationships: {}", total_relationships / 2)); // Divide by 2 since bidirectional

            if let Some(selected) = state.relationship_graph_state.selected_agent {
                ui.label(format!("| Selected: {:.8}...", selected));
            }
        } else {
            ui.label("No data");
        }
    });
}

/// Compute initial layout based on layout mode
fn compute_layout(state: &mut GuiState, snapshot: &RelationshipGraphSnapshot, rect: Rect) {
    state.relationship_graph_state.node_positions.clear();
    state.relationship_graph_state.layout_iterations = 0;

    let nodes = &snapshot.nodes;
    let node_count = nodes.len();

    if node_count == 0 {
        return;
    }

    // Filter to focus agent's connections if set
    let relevant_nodes: Vec<&RelationshipGraphNode> = if let Some(focus_id) = state.relationship_graph_state.focus_agent {
        let connected: std::collections::HashSet<Uuid> = nodes.iter()
            .find(|n| n.agent_id == focus_id)
            .map(|n| {
                let mut set: std::collections::HashSet<Uuid> = n.relationships.iter()
                    .map(|r| r.target_id)
                    .collect();
                set.insert(focus_id);
                set
            })
            .unwrap_or_default();

        nodes.iter().filter(|n| connected.contains(&n.agent_id)).collect()
    } else {
        nodes.iter().collect()
    };

    let count = relevant_nodes.len();
    let spread = (rect.width().min(rect.height()) * 0.35) as f32;

    match state.relationship_graph_state.layout_mode {
        GraphLayoutMode::Circular => {
            for (i, node) in relevant_nodes.iter().enumerate() {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let x = angle.cos() * spread;
                let y = angle.sin() * spread;
                state.relationship_graph_state.node_positions.insert(
                    node.agent_id,
                    GraphNodePosition { x, y, vx: 0.0, vy: 0.0 }
                );
            }
        }
        GraphLayoutMode::Spatial => {
            let min_x = relevant_nodes.iter().map(|n| n.position.0).min().unwrap_or(0) as f32;
            let max_x = relevant_nodes.iter().map(|n| n.position.0).max().unwrap_or(100) as f32;
            let min_y = relevant_nodes.iter().map(|n| n.position.1).min().unwrap_or(0) as f32;
            let max_y = relevant_nodes.iter().map(|n| n.position.1).max().unwrap_or(100) as f32;

            let scale_x = if max_x > min_x { spread * 2.0 / (max_x - min_x) } else { 1.0 };
            let scale_y = if max_y > min_y { spread * 2.0 / (max_y - min_y) } else { 1.0 };
            let scale = scale_x.min(scale_y);

            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

            for node in relevant_nodes {
                let x = (node.position.0 as f32 - center_x) * scale;
                let y = (node.position.1 as f32 - center_y) * scale;
                state.relationship_graph_state.node_positions.insert(
                    node.agent_id,
                    GraphNodePosition { x, y, vx: 0.0, vy: 0.0 }
                );
            }
        }
        GraphLayoutMode::ForceDirected => {
            // Initialize with circular layout, then force-directed will refine
            for (i, node) in relevant_nodes.iter().enumerate() {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let r = spread * 0.5 + (i as f32 % 3.0) * spread * 0.2;
                let x = angle.cos() * r;
                let y = angle.sin() * r;
                state.relationship_graph_state.node_positions.insert(
                    node.agent_id,
                    GraphNodePosition { x, y, vx: 0.0, vy: 0.0 }
                );
            }
        }
    }
}

/// Run one iteration of force-directed layout
fn run_force_directed_iteration(state: &mut GuiState, snapshot: &RelationshipGraphSnapshot) {
    let damping = 0.85;
    let repulsion = 5000.0;
    let attraction = 0.05;
    let center_pull = 0.01;

    let node_ids: Vec<Uuid> = state.relationship_graph_state.node_positions.keys().copied().collect();

    // Calculate forces
    let mut forces: HashMap<Uuid, (f32, f32)> = HashMap::new();
    for id in &node_ids {
        forces.insert(*id, (0.0, 0.0));
    }

    // Repulsion between all nodes
    for i in 0..node_ids.len() {
        for j in (i + 1)..node_ids.len() {
            let id1 = node_ids[i];
            let id2 = node_ids[j];

            let pos1 = state.relationship_graph_state.node_positions.get(&id1).unwrap();
            let pos2 = state.relationship_graph_state.node_positions.get(&id2).unwrap();

            let dx = pos2.x - pos1.x;
            let dy = pos2.y - pos1.y;
            let dist_sq = dx * dx + dy * dy + 1.0;
            let dist = dist_sq.sqrt();

            let force = repulsion / dist_sq;
            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            if let Some(f) = forces.get_mut(&id1) {
                f.0 -= fx;
                f.1 -= fy;
            }
            if let Some(f) = forces.get_mut(&id2) {
                f.0 += fx;
                f.1 += fy;
            }
        }
    }

    // Attraction along edges
    for node in &snapshot.nodes {
        if !state.relationship_graph_state.node_positions.contains_key(&node.agent_id) {
            continue;
        }

        for edge in &node.relationships {
            if !state.relationship_graph_state.node_positions.contains_key(&edge.target_id) {
                continue;
            }

            let pos1 = state.relationship_graph_state.node_positions.get(&node.agent_id).unwrap();
            let pos2 = state.relationship_graph_state.node_positions.get(&edge.target_id).unwrap();

            let dx = pos2.x - pos1.x;
            let dy = pos2.y - pos1.y;
            let dist = (dx * dx + dy * dy).sqrt() + 1.0;

            // Strong bonds attract more, negative bonds repel
            let bond_factor = edge.bond_strength;
            let force = attraction * dist * bond_factor;

            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            if let Some(f) = forces.get_mut(&node.agent_id) {
                f.0 += fx;
                f.1 += fy;
            }
        }
    }

    // Center pull
    for id in &node_ids {
        let pos = state.relationship_graph_state.node_positions.get(id).unwrap();
        if let Some(f) = forces.get_mut(id) {
            f.0 -= pos.x * center_pull;
            f.1 -= pos.y * center_pull;
        }
    }

    // Apply forces with damping
    for id in &node_ids {
        let (fx, fy) = forces.get(id).copied().unwrap_or((0.0, 0.0));
        if let Some(pos) = state.relationship_graph_state.node_positions.get_mut(id) {
            pos.vx = (pos.vx + fx) * damping;
            pos.vy = (pos.vy + fy) * damping;
            pos.x += pos.vx;
            pos.y += pos.vy;
        }
    }
}

/// Check if an edge should be shown based on filters
fn should_show_edge(edge: &RelationshipEdge, filter: &RelationshipFilter) -> bool {
    if edge.bond_strength < filter.min_bond_strength {
        return false;
    }

    match edge.relationship_type.as_str() {
        "Parent" => filter.show_parent,
        "Child" => filter.show_child,
        "Sibling" => filter.show_sibling,
        "Partner" => filter.show_partner,
        "Friend" => filter.show_friend,
        "Acquaintance" => filter.show_acquaintance,
        "Rival" => filter.show_rival,
        "Enemy" => filter.show_enemy,
        _ => true,
    }
}

/// Get color for relationship type
fn relationship_type_color(rel_type: &str) -> Color32 {
    match rel_type {
        "Parent" => Color32::from_rgb(255, 182, 193),    // Pink
        "Child" => Color32::from_rgb(255, 182, 193),     // Pink
        "Sibling" => Color32::from_rgb(200, 150, 200),   // Light purple
        "Partner" => Color32::from_rgb(255, 105, 180),   // Hot pink
        "Friend" => Color32::from_rgb(100, 200, 100),    // Green
        "Acquaintance" => Color32::from_rgb(150, 150, 150), // Gray
        "Rival" => Color32::from_rgb(255, 165, 0),       // Orange
        "Enemy" => Color32::from_rgb(255, 50, 50),       // Red
        _ => Color32::GRAY,
    }
}

/// Get edge thickness from bond strength
fn edge_thickness(bond_strength: f32) -> f32 {
    let normalized = (bond_strength.abs() + 1.0) / 2.0;
    MIN_EDGE_THICKNESS + normalized * (MAX_EDGE_THICKNESS - MIN_EDGE_THICKNESS)
}

/// Get color for agent life stage
fn life_stage_color(life_stage: LifeStage) -> Color32 {
    match life_stage {
        LifeStage::Infant => Color32::from_rgb(255, 220, 180),  // Light peach
        LifeStage::Child => Color32::from_rgb(180, 220, 255),   // Light blue
        LifeStage::Adolescent => Color32::from_rgb(180, 255, 200), // Light green
        LifeStage::Adult => Color32::from_rgb(100, 150, 255),   // Blue
        LifeStage::Elderly => Color32::from_rgb(200, 180, 255), // Light purple
    }
}

/// Get color for health percentage
fn health_to_color(health: f32) -> Color32 {
    let h = (health / 100.0).clamp(0.0, 1.0);
    if h > 0.6 {
        Color32::from_rgb(50, 200, 50)  // Green
    } else if h > 0.3 {
        Color32::from_rgb(255, 200, 50) // Yellow
    } else {
        Color32::from_rgb(255, 50, 50)  // Red
    }
}
