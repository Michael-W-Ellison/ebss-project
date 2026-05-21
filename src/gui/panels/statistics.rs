// src/gui/panels/statistics.rs
//! Population and world statistics panel.

use egui::{Ui, Color32, ProgressBar};
use crate::gui::state::GuiState;

pub fn render_statistics(ui: &mut Ui, state: &GuiState) {
    ui.heading("Statistics");
    ui.separator();

    let Some(snapshot) = &state.latest_snapshot else {
        ui.label("Waiting for simulation data...");
        return;
    };

    let stats = &snapshot.population.stats;

    // Population overview
    ui.collapsing("Population", |ui| {
        ui.label(format!("Total Agents: {}", stats.total_agents));

        ui.add_space(5.0);
        ui.label("Life Stages:");

        // Life stage breakdown
        let total = stats.total_agents.max(1) as f32;

        ui.horizontal(|ui| {
            ui.label("Infants:");
            ui.add(ProgressBar::new(stats.infants as f32 / total)
                .fill(Color32::from_rgb(255, 182, 193))
                .text(format!("{}", stats.infants)));
        });

        ui.horizontal(|ui| {
            ui.label("Children:");
            ui.add(ProgressBar::new(stats.children as f32 / total)
                .fill(Color32::from_rgb(135, 206, 250))
                .text(format!("{}", stats.children)));
        });

        ui.horizontal(|ui| {
            ui.label("Adolescents:");
            ui.add(ProgressBar::new(stats.adolescents as f32 / total)
                .fill(Color32::from_rgb(144, 238, 144))
                .text(format!("{}", stats.adolescents)));
        });

        ui.horizontal(|ui| {
            ui.label("Adults:");
            ui.add(ProgressBar::new(stats.adults as f32 / total)
                .fill(Color32::WHITE)
                .text(format!("{}", stats.adults)));
        });

        ui.horizontal(|ui| {
            ui.label("Elderly:");
            ui.add(ProgressBar::new(stats.elderly as f32 / total)
                .fill(Color32::from_rgb(192, 192, 192))
                .text(format!("{}", stats.elderly)));
        });

        ui.add_space(10.0);
        ui.label("Lifetime Events:");
        ui.label(format!("  Births: {}", stats.total_births));
        ui.label(format!("  Deaths: {}", stats.total_deaths));
    });

    ui.add_space(5.0);

    // Health & Energy
    ui.collapsing("Health & Energy", |ui| {
        ui.horizontal(|ui| {
            ui.label("Avg Health:");
            let health_color = if stats.average_health > 70.0 {
                Color32::GREEN
            } else if stats.average_health > 30.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.add(ProgressBar::new(stats.average_health / 100.0)
                .fill(health_color)
                .text(format!("{:.1}%", stats.average_health)));
        });

        ui.horizontal(|ui| {
            ui.label("Avg Energy:");
            let energy_color = if stats.average_energy > 50.0 {
                Color32::from_rgb(0, 200, 255)
            } else if stats.average_energy > 20.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.add(ProgressBar::new(stats.average_energy / 100.0)
                .fill(energy_color)
                .text(format!("{:.1}%", stats.average_energy)));
        });

        ui.horizontal(|ui| {
            ui.label("Avg Happiness:");
            let happiness_color = if stats.average_happiness > 0.6 {
                Color32::GREEN
            } else if stats.average_happiness > 0.3 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            ui.add(ProgressBar::new(stats.average_happiness)
                .fill(happiness_color)
                .text(format!("{:.1}%", stats.average_happiness * 100.0)));
        });
    });

    ui.add_space(5.0);

    // World resources
    ui.collapsing("World Resources", |ui| {
        let world = &snapshot.world;

        // Count resources by type
        let mut resource_counts: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();

        for resource in &world.resources {
            let key = format!("{:?}", resource.resource_type);
            let entry = resource_counts.entry(key).or_insert((0, 0));
            entry.0 += resource.amount;
            entry.1 += resource.max_amount;
        }

        ui.label(format!("Resource Nodes: {}", world.resources.len()));
        ui.add_space(5.0);

        for (resource_type, (amount, max_amount)) in resource_counts.iter() {
            let pct = *amount as f32 / (*max_amount).max(1) as f32;
            ui.horizontal(|ui| {
                ui.label(format!("{}:", resource_type));
                ui.add(ProgressBar::new(pct)
                    .text(format!("{}/{}", amount, max_amount)));
            });
        }
    });

    ui.add_space(5.0);

    // Buildings
    ui.collapsing("Buildings", |ui| {
        let world = &snapshot.world;

        let completed = world.buildings.iter().filter(|b| b.completed).count();
        let under_construction = world.buildings.len() - completed;

        ui.label(format!("Total: {}", world.buildings.len()));
        ui.label(format!("  Completed: {}", completed));
        ui.label(format!("  Under Construction: {}", under_construction));

        // Count by type
        let mut building_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for building in &world.buildings {
            let key = format!("{:?}", building.building_type);
            *building_counts.entry(key).or_insert(0) += 1;
        }

        if !building_counts.is_empty() {
            ui.add_space(5.0);
            ui.label("By Type:");
            for (building_type, count) in building_counts.iter() {
                ui.label(format!("  {}: {}", building_type, count));
            }
        }
    });

    ui.add_space(5.0);

    // World info
    ui.collapsing("World Info", |ui| {
        let world = &snapshot.world;
        ui.label(format!("Size: {}x{}", world.width, world.height));
        ui.label(format!("Tick: {}", world.tick));

        // Calculate in-game time
        let days = world.tick / 1440;
        let hours = (world.tick % 1440) / 60;
        let minutes = world.tick % 60;
        ui.label(format!("Time: Day {} {:02}:{:02}", days + 1, hours, minutes));
    });
}
