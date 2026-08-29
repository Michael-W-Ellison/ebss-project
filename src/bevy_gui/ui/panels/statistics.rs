// src/bevy_gui/ui/panels/statistics.rs
//! Population and world statistics panel with real-time graphs.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_plot::{Plot, Line, PlotPoints, Legend, Corner};

use crate::bevy_gui::resources::{
    PanelVisibility, CurrentSnapshot, StatisticsHistory, StatisticsTab,
};

const GRAPH_HEIGHT: f32 = 150.0;

pub fn render_statistics_panel(
    mut egui_ctx: EguiContexts,
    panels: Res<PanelVisibility>,
    snapshot: Res<CurrentSnapshot>,
    mut history: ResMut<StatisticsHistory>,
) {
    if !panels.statistics {
        return;
    }

    egui::SidePanel::right("statistics_panel")
        .default_width(350.0)
        .resizable(true)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Statistics");
            ui.separator();

            let Some(snap) = &snapshot.snapshot else {
                ui.label("Waiting for simulation data...");
                return;
            };

            // Quick stats header
            render_quick_stats(ui, snap);
            ui.separator();

            // Tab bar
            ui.horizontal(|ui| {
                let tabs = [
                    (StatisticsTab::Population, "Population"),
                    (StatisticsTab::Health, "Vitals"),
                    (StatisticsTab::Resources, "Resources"),
                    (StatisticsTab::Economy, "Economy"),
                ];
                for (tab, label) in tabs {
                    if ui.selectable_label(history.active_tab == tab, label).clicked() {
                        history.active_tab = tab;
                    }
                }
            });
            ui.separator();

            // Tab content
            egui::ScrollArea::vertical().show(ui, |ui| {
                match history.active_tab {
                    StatisticsTab::Population => render_population_tab(ui, &history, snap),
                    StatisticsTab::Resources => render_resources_tab(ui, snap),
                    StatisticsTab::Economy => render_economy_tab(ui, snap),
                    StatisticsTab::Health => render_vitals_tab(ui, &history, snap),
                }
            });
        });
}

fn render_quick_stats(ui: &mut egui::Ui, snapshot: &crate::gui::state::SimulationSnapshot) {
    let stats = &snapshot.population.stats;
    let world = &snapshot.world;

    ui.horizontal(|ui| {
        // Population
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(format!("{}", stats.total_agents)).size(20.0).strong());
            ui.label(egui::RichText::new("Population").small());
        });

        ui.separator();

        // Tick/Time
        ui.vertical(|ui| {
            let days = world.tick / 1440;
            ui.label(egui::RichText::new(format!("Day {}", days + 1)).size(16.0));
            let hours = (world.tick % 1440) / 60;
            let minutes = world.tick % 60;
            ui.label(egui::RichText::new(format!("{:02}:{:02}", hours, minutes)).small());
        });

        ui.separator();

        // Health indicator
        ui.vertical(|ui| {
            let color = vitals_color(stats.average_health);
            ui.label(egui::RichText::new(format!("{:.0}%", stats.average_health)).size(16.0).color(color));
            ui.label(egui::RichText::new("Health").small());
        });

        ui.separator();

        // Energy indicator
        ui.vertical(|ui| {
            let color = vitals_color(stats.average_energy);
            ui.label(egui::RichText::new(format!("{:.0}%", stats.average_energy)).size(16.0).color(color));
            ui.label(egui::RichText::new("Energy").small());
        });
    });
}

fn render_population_tab(
    ui: &mut egui::Ui,
    history: &StatisticsHistory,
    snapshot: &crate::gui::state::SimulationSnapshot,
) {
    let stats = &snapshot.population.stats;

    // Population graph
    ui.heading("Population Over Time");
    if history.points.len() >= 2 {
        let population_data = history.population_data();
        Plot::new("population_graph")
            .height(GRAPH_HEIGHT)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(population_data))
                        .color(egui::Color32::from_rgb(100, 200, 255))
                        .name("Population")
                );
            });
    } else {
        ui.label("Collecting data...");
    }

    ui.add_space(10.0);

    // Population breakdown
    ui.heading("Population Breakdown");
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("By Life Stage").strong());
            stage_stat(ui, "Infants", stats.infants, stats.total_agents);
            stage_stat(ui, "Children", stats.children, stats.total_agents);
            stage_stat(ui, "Adolescents", stats.adolescents, stats.total_agents);
            stage_stat(ui, "Adults", stats.adults, stats.total_agents);
            stage_stat(ui, "Elderly", stats.elderly, stats.total_agents);
        });
    });

    ui.add_space(10.0);

    // Births and deaths
    ui.heading("Demographics");
    ui.horizontal(|ui| {
        ui.label(format!("Total Births: {}", stats.total_births));
        ui.separator();
        ui.label(format!("Total Deaths: {}", stats.total_deaths));
    });

    let growth = stats.total_births as i64 - stats.total_deaths as i64;
    let growth_color = if growth > 0 {
        egui::Color32::GREEN
    } else if growth < 0 {
        egui::Color32::RED
    } else {
        egui::Color32::GRAY
    };
    ui.colored_label(growth_color, format!("Net Growth: {:+}", growth));
}

fn render_vitals_tab(
    ui: &mut egui::Ui,
    history: &StatisticsHistory,
    snapshot: &crate::gui::state::SimulationSnapshot,
) {
    let stats = &snapshot.population.stats;

    // Current vitals
    ui.heading("Current Averages");
    vitals_bar(ui, "Health", stats.average_health);
    vitals_bar(ui, "Energy", stats.average_energy);
    vitals_bar(ui, "Happiness", stats.average_happiness);

    ui.add_space(10.0);

    // Vitals history graph
    ui.heading("Vitals Over Time");
    if history.points.len() >= 2 {
        Plot::new("vitals_graph")
            .height(GRAPH_HEIGHT)
            .legend(Legend::default().position(Corner::LeftTop))
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.health_data()))
                        .color(egui::Color32::from_rgb(100, 200, 100))
                        .name("Health")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.energy_data()))
                        .color(egui::Color32::from_rgb(100, 150, 255))
                        .name("Energy")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.happiness_data()))
                        .color(egui::Color32::from_rgb(255, 200, 100))
                        .name("Happiness")
                );
            });
    } else {
        ui.label("Collecting data...");
    }

    ui.add_space(10.0);

    // Critical status
    ui.heading("Status Alerts");
    let critical_count = snapshot.population.agents.iter()
        .filter(|a| a.is_alive && a.health < 25.0)
        .count();
    let starving_count = snapshot.population.agents.iter()
        .filter(|a| a.is_alive && a.energy < 20.0)
        .count();

    if critical_count > 0 {
        ui.colored_label(
            egui::Color32::RED,
            format!("⚠ {} agents in critical health", critical_count)
        );
    }
    if starving_count > 0 {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!("⚠ {} agents with low energy", starving_count)
        );
    }
    if critical_count == 0 && starving_count == 0 {
        ui.colored_label(egui::Color32::GREEN, "✓ All agents in good condition");
    }
}

fn render_resources_tab(
    ui: &mut egui::Ui,
    snapshot: &crate::gui::state::SimulationSnapshot,
) {
    ui.heading("World Resources");

    let resources = &snapshot.world.resources;
    if resources.is_empty() {
        ui.label("No resources tracked.");
        return;
    }

    // Count resources by type
    let mut resource_counts: std::collections::BTreeMap<String, (u32, u32)> = std::collections::BTreeMap::new();
    for resource in resources {
        let entry = resource_counts.entry(format!("{:?}", resource.resource_type)).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += resource.amount;
    }

    let mut sorted_resources: Vec<_> = resource_counts.into_iter().collect();
    sorted_resources.sort_by(|a, b| b.1.1.cmp(&a.1.1));

    for (name, (count, total)) in sorted_resources {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&name).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("x{}", count));
                });
            });
            ui.label(format!("Total: {}", total));
        });
    }

    ui.add_space(10.0);

    // Buildings summary
    ui.heading("Buildings");
    let buildings = &snapshot.world.buildings;
    let completed = buildings.iter().filter(|b| b.progress >= 1.0).count();
    let in_progress = buildings.len() - completed;

    ui.label(format!("Completed: {}", completed));
    ui.label(format!("Under Construction: {}", in_progress));
}

fn render_economy_tab(
    ui: &mut egui::Ui,
    snapshot: &crate::gui::state::SimulationSnapshot,
) {
    ui.heading("Economy");

    // Count total inventory items
    let total_items: u32 = snapshot.population.agents.iter()
        .map(|a| a.inventory_count)
        .sum();

    ui.label(format!("Total Items in Circulation: {}", total_items));

    ui.add_space(10.0);

    // Activity breakdown
    ui.heading("Agent Activities");

    let mut activities: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for agent in &snapshot.population.agents {
        if let Some(activity) = &agent.current_activity {
            let key = if activity.len() > 20 {
                format!("{}...", &activity[..20])
            } else {
                activity.clone()
            };
            *activities.entry(key).or_insert(0) += 1;
        } else {
            *activities.entry("Idle".to_string()).or_insert(0) += 1;
        }
    }

    let mut sorted_activities: Vec<_> = activities.into_iter().collect();
    sorted_activities.sort_by(|a, b| b.1.cmp(&a.1));

    for (activity, count) in sorted_activities.into_iter().take(10) {
        ui.horizontal(|ui| {
            ui.label(&activity);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("{}", count));
            });
        });
    }
}

// Helper functions

fn stage_stat(ui: &mut egui::Ui, label: &str, count: usize, total: usize) {
    let pct = if total > 0 { count as f32 / total as f32 * 100.0 } else { 0.0 };
    ui.horizontal(|ui| {
        ui.label(format!("{}: {}", label, count));
        ui.label(egui::RichText::new(format!("({:.0}%)", pct)).small().color(egui::Color32::GRAY));
    });
}

fn vitals_bar(ui: &mut egui::Ui, label: &str, value: f32) {
    ui.horizontal(|ui| {
        ui.label(format!("{}:", label));
        let color = vitals_color(value);
        ui.add(egui::ProgressBar::new(value / 100.0)
            .fill(color)
            .text(format!("{:.0}%", value)));
    });
}

fn vitals_color(value: f32) -> egui::Color32 {
    if value >= 75.0 {
        egui::Color32::from_rgb(100, 200, 100)
    } else if value >= 50.0 {
        egui::Color32::from_rgb(200, 200, 100)
    } else if value >= 25.0 {
        egui::Color32::from_rgb(200, 150, 100)
    } else {
        egui::Color32::from_rgb(200, 100, 100)
    }
}
