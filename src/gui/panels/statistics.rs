// src/gui/panels/statistics.rs
//! Population and world statistics panel with real-time graphs.

use egui::{Ui, Color32, ProgressBar, RichText, ScrollArea};
use egui_plot::{Plot, Line, PlotPoints, Legend, Corner};
use crate::gui::state::{GuiState, StatisticsTab};

const GRAPH_HEIGHT: f32 = 150.0;

pub fn render_statistics(ui: &mut Ui, state: &mut GuiState) {
    ui.heading("Statistics");
    ui.separator();

    let Some(snapshot) = &state.latest_snapshot else {
        ui.label("Waiting for simulation data...");
        return;
    };

    // Quick stats header
    render_quick_stats(ui, snapshot);
    ui.separator();

    // Tab bar
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.statistics_tab, StatisticsTab::Overview, "Overview");
        ui.selectable_value(&mut state.statistics_tab, StatisticsTab::Population, "Population");
        ui.selectable_value(&mut state.statistics_tab, StatisticsTab::Vitals, "Vitals");
        ui.selectable_value(&mut state.statistics_tab, StatisticsTab::Resources, "Resources");
        ui.selectable_value(&mut state.statistics_tab, StatisticsTab::Buildings, "Buildings");
    });
    ui.separator();

    // Tab content
    ScrollArea::vertical().show(ui, |ui| {
        match state.statistics_tab {
            StatisticsTab::Overview => render_overview_tab(ui, state),
            StatisticsTab::Population => render_population_tab(ui, state),
            StatisticsTab::Vitals => render_vitals_tab(ui, state),
            StatisticsTab::Resources => render_resources_tab(ui, state),
            StatisticsTab::Buildings => render_buildings_tab(ui, state),
        }
    });
}

fn render_quick_stats(ui: &mut Ui, snapshot: &crate::gui::state::SimulationSnapshot) {
    let stats = &snapshot.population.stats;
    let world = &snapshot.world;

    ui.horizontal(|ui| {
        // Population
        ui.vertical(|ui| {
            ui.label(RichText::new(format!("{}", stats.total_agents)).size(20.0).strong());
            ui.label(RichText::new("Population").small());
        });

        ui.separator();

        // Tick/Time
        ui.vertical(|ui| {
            let (days, hours, minutes) =
                crate::environment::seasons::what_the_clock_says(world.tick);
            ui.label(RichText::new(format!("Day {}", days + 1)).size(16.0));
            ui.label(RichText::new(format!("{:02}:{:02}", hours, minutes)).small());
        });

        ui.separator();

        // Health indicator
        ui.vertical(|ui| {
            let color = vitals_color(stats.average_health);
            ui.label(RichText::new(format!("{:.0}%", stats.average_health)).size(16.0).color(color));
            ui.label(RichText::new("Health").small());
        });
    });
}

// ============================================================================
// OVERVIEW TAB
// ============================================================================

fn render_overview_tab(ui: &mut Ui, state: &GuiState) {
    let Some(snapshot) = &state.latest_snapshot else { return };
    let stats = &snapshot.population.stats;
    let history = &state.statistics_history;

    // Population graph
    ui.heading("Population Over Time");
    if history.points.len() >= 2 {
        let population_data = history.population_data();
        Plot::new("population_overview")
            .height(GRAPH_HEIGHT)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(population_data))
                        .color(Color32::from_rgb(100, 200, 255))
                        .name("Population")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.add_space(10.0);

    // Current stats summary
    ui.heading("Current Statistics");

    ui.columns(2, |cols| {
        // Left column
        cols[0].group(|ui| {
            ui.label(RichText::new("Population").strong());
            ui.label(format!("Total: {}", stats.total_agents));
            ui.label(format!("Births: {}", stats.total_births));
            ui.label(format!("Deaths: {}", stats.total_deaths));
        });

        // Right column
        cols[1].group(|ui| {
            ui.label(RichText::new("Averages").strong());
            ui.horizontal(|ui| {
                ui.label("Health:");
                ui.add(ProgressBar::new(stats.average_health / 100.0)
                    .fill(vitals_color(stats.average_health))
                    .desired_width(60.0));
            });
            ui.horizontal(|ui| {
                ui.label("Energy:");
                ui.add(ProgressBar::new(stats.average_energy / 100.0)
                    .fill(vitals_color(stats.average_energy))
                    .desired_width(60.0));
            });
        });
    });
}

// ============================================================================
// POPULATION TAB
// ============================================================================

fn render_population_tab(ui: &mut Ui, state: &GuiState) {
    let Some(snapshot) = &state.latest_snapshot else { return };
    let stats = &snapshot.population.stats;
    let history = &state.statistics_history;

    // Total population graph
    ui.heading("Total Population");
    if history.points.len() >= 2 {
        Plot::new("population_total")
            .height(GRAPH_HEIGHT)
            .legend(Legend::default().position(Corner::LeftTop))
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.population_data()))
                        .color(Color32::WHITE)
                        .name("Total")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.add_space(10.0);

    // Life stages graph
    ui.heading("Life Stages");
    if history.points.len() >= 2 {
        Plot::new("life_stages")
            .height(GRAPH_HEIGHT + 30.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.life_stage_data("infants")))
                        .color(Color32::from_rgb(255, 182, 193))
                        .name("Infants")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.life_stage_data("children")))
                        .color(Color32::from_rgb(135, 206, 250))
                        .name("Children")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.life_stage_data("adolescents")))
                        .color(Color32::from_rgb(144, 238, 144))
                        .name("Adolescents")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.life_stage_data("adults")))
                        .color(Color32::WHITE)
                        .name("Adults")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.life_stage_data("elderly")))
                        .color(Color32::from_rgb(192, 192, 192))
                        .name("Elderly")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.add_space(10.0);

    // Current breakdown
    ui.heading("Current Breakdown");
    let total = stats.total_agents.max(1) as f32;

    render_life_stage_bar(ui, "Infants", stats.infants, total, Color32::from_rgb(255, 182, 193));
    render_life_stage_bar(ui, "Children", stats.children, total, Color32::from_rgb(135, 206, 250));
    render_life_stage_bar(ui, "Adolescents", stats.adolescents, total, Color32::from_rgb(144, 238, 144));
    render_life_stage_bar(ui, "Adults", stats.adults, total, Color32::WHITE);
    render_life_stage_bar(ui, "Elderly", stats.elderly, total, Color32::from_rgb(192, 192, 192));

    ui.add_space(10.0);

    // Births and Deaths
    ui.heading("Births & Deaths");
    if history.points.len() >= 2 {
        let (births, deaths) = history.births_deaths_data();
        Plot::new("births_deaths")
            .height(GRAPH_HEIGHT)
            .legend(Legend::default().position(Corner::LeftTop))
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(births))
                        .color(Color32::from_rgb(100, 255, 100))
                        .name("Births")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(deaths))
                        .color(Color32::from_rgb(255, 100, 100))
                        .name("Deaths")
                );
            });
    }

    ui.horizontal(|ui| {
        ui.colored_label(Color32::from_rgb(100, 255, 100), format!("Total Births: {}", stats.total_births));
        ui.separator();
        ui.colored_label(Color32::from_rgb(255, 100, 100), format!("Total Deaths: {}", stats.total_deaths));
    });
}

fn render_life_stage_bar(ui: &mut Ui, name: &str, count: usize, total: f32, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(format!("{:12}", name));
        ui.add(ProgressBar::new(count as f32 / total)
            .fill(color)
            .desired_width(100.0)
            .text(format!("{} ({:.0}%)", count, count as f32 / total * 100.0)));
    });
}

// ============================================================================
// VITALS TAB
// ============================================================================

fn render_vitals_tab(ui: &mut Ui, state: &GuiState) {
    let Some(snapshot) = &state.latest_snapshot else { return };
    let stats = &snapshot.population.stats;
    let history = &state.statistics_history;

    // Health graph
    ui.heading("Average Health");
    if history.points.len() >= 2 {
        Plot::new("health_graph")
            .height(GRAPH_HEIGHT)
            .include_y(0.0)
            .include_y(100.0)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.health_data()))
                        .color(Color32::from_rgb(255, 100, 100))
                        .name("Health %")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.horizontal(|ui| {
        ui.label("Current:");
        ui.add(ProgressBar::new(stats.average_health / 100.0)
            .fill(vitals_color(stats.average_health))
            .text(format!("{:.1}%", stats.average_health)));
    });

    ui.add_space(15.0);

    // Energy graph
    ui.heading("Average Energy");
    if history.points.len() >= 2 {
        Plot::new("energy_graph")
            .height(GRAPH_HEIGHT)
            .include_y(0.0)
            .include_y(100.0)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.energy_data()))
                        .color(Color32::from_rgb(100, 200, 255))
                        .name("Energy %")
                );
            });
    }

    ui.horizontal(|ui| {
        ui.label("Current:");
        ui.add(ProgressBar::new(stats.average_energy / 100.0)
            .fill(vitals_color(stats.average_energy))
            .text(format!("{:.1}%", stats.average_energy)));
    });

    ui.add_space(15.0);

    // Happiness graph
    ui.heading("Average Happiness");
    if history.points.len() >= 2 {
        Plot::new("happiness_graph")
            .height(GRAPH_HEIGHT)
            .include_y(0.0)
            .include_y(100.0)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.happiness_data()))
                        .color(Color32::from_rgb(255, 215, 0))
                        .name("Happiness %")
                );
            });
    }

    ui.horizontal(|ui| {
        ui.label("Current:");
        let happiness_pct = stats.average_happiness * 100.0;
        ui.add(ProgressBar::new(stats.average_happiness)
            .fill(happiness_color(stats.average_happiness))
            .text(format!("{:.1}%", happiness_pct)));
    });

    ui.add_space(15.0);

    // Combined vitals graph
    ui.heading("All Vitals");
    if history.points.len() >= 2 {
        Plot::new("all_vitals")
            .height(GRAPH_HEIGHT + 20.0)
            .legend(Legend::default().position(Corner::LeftTop))
            .include_y(0.0)
            .include_y(100.0)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.health_data()))
                        .color(Color32::from_rgb(255, 100, 100))
                        .name("Health")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.energy_data()))
                        .color(Color32::from_rgb(100, 200, 255))
                        .name("Energy")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(history.happiness_data()))
                        .color(Color32::from_rgb(255, 215, 0))
                        .name("Happiness")
                );
            });
    }
}

// ============================================================================
// RESOURCES TAB
// ============================================================================

fn render_resources_tab(ui: &mut Ui, state: &GuiState) {
    let Some(snapshot) = &state.latest_snapshot else { return };
    let world = &snapshot.world;
    let history = &state.statistics_history;

    // Total resources graph
    ui.heading("Total Resources");
    if history.points.len() >= 2 {
        Plot::new("resources_total")
            .height(GRAPH_HEIGHT)
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(history.resources_data()))
                        .color(Color32::from_rgb(255, 200, 100))
                        .name("Total Resources")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.add_space(10.0);

    // Resource breakdown
    ui.heading("Resource Breakdown");

    // Count resources by type
    let mut resource_counts: std::collections::BTreeMap<String, (u32, u32)> = std::collections::BTreeMap::new();
    for resource in &world.resources {
        let key = format!("{:?}", resource.resource_type);
        let entry = resource_counts.entry(key).or_insert((0, 0));
        entry.0 += resource.amount;
        entry.1 += resource.max_amount;
    }

    // Sort by amount
    let mut sorted: Vec<_> = resource_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.0.cmp(&a.1.0));

    ui.label(format!("Resource Nodes: {}", world.resources.len()));
    ui.add_space(5.0);

    for (resource_type, (amount, max_amount)) in sorted.iter().take(10) {
        let pct = *amount as f32 / (*max_amount).max(1) as f32;
        let color = resource_fill_color(pct);
        ui.horizontal(|ui| {
            ui.label(format!("{:12}", resource_type));
            ui.add(ProgressBar::new(pct)
                .fill(color)
                .desired_width(80.0)
                .text(format!("{}/{}", amount, max_amount)));
        });
    }

    if sorted.len() > 10 {
        ui.label(format!("... and {} more types", sorted.len() - 10));
    }

    ui.add_space(10.0);

    // Resource summary
    ui.heading("Summary");
    let total_amount: u32 = resource_counts.values().map(|(a, _)| a).sum();
    let total_max: u32 = resource_counts.values().map(|(_, m)| m).sum();
    let overall_pct = total_amount as f32 / total_max.max(1) as f32;

    ui.horizontal(|ui| {
        ui.label("Overall:");
        ui.add(ProgressBar::new(overall_pct)
            .fill(resource_fill_color(overall_pct))
            .text(format!("{:.1}% ({}/{})", overall_pct * 100.0, total_amount, total_max)));
    });

    let depleted = world.resources.iter().filter(|r| r.amount == 0).count();
    let low = world.resources.iter().filter(|r| r.amount > 0 && (r.amount as f32 / r.max_amount as f32) < 0.25).count();

    ui.horizontal(|ui| {
        if depleted > 0 {
            ui.colored_label(Color32::RED, format!("Depleted: {}", depleted));
        }
        if low > 0 {
            ui.colored_label(Color32::YELLOW, format!("Low: {}", low));
        }
        if depleted == 0 && low == 0 {
            ui.colored_label(Color32::GREEN, "All resources healthy");
        }
    });
}

// ============================================================================
// BUILDINGS TAB
// ============================================================================

fn render_buildings_tab(ui: &mut Ui, state: &GuiState) {
    let Some(snapshot) = &state.latest_snapshot else { return };
    let world = &snapshot.world;
    let history = &state.statistics_history;

    // Buildings over time graph
    ui.heading("Buildings Over Time");
    if history.points.len() >= 2 {
        let (completed, construction) = history.buildings_data();
        Plot::new("buildings_graph")
            .height(GRAPH_HEIGHT)
            .legend(Legend::default().position(Corner::LeftTop))
            .show_axes([false, true])
            .allow_drag(false)
            .allow_zoom(false)
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new(PlotPoints::new(completed))
                        .color(Color32::from_rgb(139, 90, 43))
                        .name("Completed")
                );
                plot_ui.line(
                    Line::new(PlotPoints::new(construction))
                        .color(Color32::from_rgb(180, 150, 100))
                        .name("Under Construction")
                );
            });
    } else {
        ui.label("Collecting data...");
        ui.add_space(GRAPH_HEIGHT);
    }

    ui.add_space(10.0);

    // Current counts
    let completed_count = world.buildings.iter().filter(|b| b.completed).count();
    let construction_count = world.buildings.len() - completed_count;

    ui.heading("Current Status");
    ui.horizontal(|ui| {
        ui.label(format!("Total: {}", world.buildings.len()));
        ui.separator();
        ui.colored_label(Color32::from_rgb(139, 90, 43), format!("Completed: {}", completed_count));
        ui.separator();
        ui.colored_label(Color32::from_rgb(180, 150, 100), format!("Building: {}", construction_count));
    });

    ui.add_space(10.0);

    // Buildings by type
    ui.heading("By Type");
    let mut building_counts: std::collections::BTreeMap<String, (usize, usize)> = std::collections::BTreeMap::new();
    for building in &world.buildings {
        let key = format!("{:?}", building.building_type);
        let entry = building_counts.entry(key).or_insert((0, 0));
        if building.completed {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    // Sort by total count
    let mut sorted: Vec<_> = building_counts.iter().collect();
    sorted.sort_by(|a, b| (b.1.0 + b.1.1).cmp(&(a.1.0 + a.1.1)));

    for (building_type, (completed, under_construction)) in sorted.iter().take(8) {
        ui.horizontal(|ui| {
            ui.label(format!("{:16}", building_type));
            ui.colored_label(Color32::from_rgb(139, 90, 43), format!("{}", completed));
            if *under_construction > 0 {
                ui.colored_label(Color32::from_rgb(180, 150, 100), format!("(+{})", under_construction));
            }
        });
    }

    if sorted.len() > 8 {
        ui.label(format!("... and {} more types", sorted.len() - 8));
    }

    // Construction progress
    if construction_count > 0 {
        ui.add_space(10.0);
        ui.heading("Under Construction");

        for building in world.buildings.iter().filter(|b| !b.completed).take(5) {
            ui.horizontal(|ui| {
                ui.label(format!("{:?}", building.building_type));
                ui.add(ProgressBar::new(building.progress)
                    .fill(Color32::from_rgb(100, 200, 100))
                    .desired_width(60.0)
                    .text(format!("{:.0}%", building.progress * 100.0)));
            });
        }

        if construction_count > 5 {
            ui.label(format!("... and {} more", construction_count - 5));
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn vitals_color(value: f32) -> Color32 {
    if value > 70.0 {
        Color32::GREEN
    } else if value > 30.0 {
        Color32::YELLOW
    } else {
        Color32::RED
    }
}

fn happiness_color(value: f32) -> Color32 {
    if value > 0.6 {
        Color32::from_rgb(255, 215, 0)
    } else if value > 0.3 {
        Color32::YELLOW
    } else {
        Color32::from_rgb(255, 100, 100)
    }
}

fn resource_fill_color(pct: f32) -> Color32 {
    if pct > 0.5 {
        Color32::from_rgb(100, 200, 100)
    } else if pct > 0.25 {
        Color32::YELLOW
    } else {
        Color32::from_rgb(255, 100, 100)
    }
}
