// src/visualization/mod.rs
//! ASCII visualization for simulation state.

use crate::agents::{Agent, Population};
use crate::core::DriveType;

/// ASCII renderer for simulation visualization
pub struct AsciiRenderer {
    pub width: usize,
    pub height: usize,
}

impl AsciiRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    /// Render the complete simulation state
    pub fn render(&self, population: &Population, tick: u32) {
        self.clear_screen();
        self.render_header(tick, population.agents.len());
        self.render_world(population);
        self.render_agent_status(population);
        self.render_legend();
    }

    /// Clear the terminal screen
    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    /// Render the header with tick and population info
    fn render_header(&self, tick: u32, population_size: usize) {
        println!("╔══════════════════════════════════════════════════════════════════════╗");
        println!("║  EBSS - Emergent Behavior Society Simulator                          ║");
        println!("║  Tick: {:6}  |  Population: {:3}  |  Phase 1 Demo              ║", tick, population_size);
        println!("╚══════════════════════════════════════════════════════════════════════╝");
        println!();
    }

    /// Render the world grid with agent positions
    fn render_world(&self, population: &Population) {
        println!("┌─ World View (20x20) ─────────────────────────────────────────────────┐");

        // Create a grid
        let grid_size = 20;
        let mut grid = vec![vec!['.'; grid_size]; grid_size];

        // Place agents on the grid
        for (idx, agent) in population.agents.iter().enumerate() {
            let x = ((agent.state.position.0.abs() % grid_size as i32) as usize).min(grid_size - 1);
            let y = ((agent.state.position.1.abs() % grid_size as i32) as usize).min(grid_size - 1);

            // Use different symbols for different agents
            let symbol = match idx {
                0 => 'A',
                1 => 'B',
                2 => 'C',
                3 => 'D',
                4 => 'E',
                _ => '*',
            };
            grid[y][x] = symbol;
        }

        // Render the grid
        for row in grid {
            print!("│ ");
            for cell in row {
                print!("{} ", cell);
            }
            println!("│");
        }

        println!("└───────────────────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Render agent status panels
    fn render_agent_status(&self, population: &Population) {
        println!("┌─ Agent Status ────────────────────────────────────────────────────────┐");

        for (idx, agent) in population.agents.iter().enumerate().take(5) {
            self.render_single_agent(agent, idx);
        }

        println!("└───────────────────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Render a single agent's status
    fn render_single_agent(&self, agent: &Agent, idx: usize) {
        let symbol = match idx {
            0 => 'A',
            1 => 'B',
            2 => 'C',
            3 => 'D',
            4 => 'E',
            _ => '*',
        };

        // Get most urgent drive
        let urgent_drive = agent.drives.most_urgent();
        let urgent_name = urgent_drive.map(|d| format!("{:?}", d.drive_type)).unwrap_or("None".to_string());
        let urgent_value = urgent_drive.map(|d| d.value).unwrap_or(0.0);

        println!("│");
        println!("│ Agent {} [Health: {:3.0}% | Energy: {:3.0}%]  Current Drive: {} ({:.2})",
            symbol,
            agent.state.health,
            agent.state.energy,
            urgent_name,
            urgent_value
        );

        // Render key drives as bars
        self.render_drive_bar(agent, DriveType::Hunger, "Hunger    ");
        self.render_drive_bar(agent, DriveType::Rest, "Rest      ");
        self.render_drive_bar(agent, DriveType::Curiosity, "Curiosity ");

        // Show behavior tree stats
        if let Some(tree) = agent.behavior_trees.first() {
            let success_rate = tree.success_rate();
            println!("│   Learning: {:5} executions, {:.1}% success rate",
                tree.total_executions,
                success_rate * 100.0
            );
        }
    }

    /// Render a drive level as a progress bar
    fn render_drive_bar(&self, agent: &Agent, drive_type: DriveType, label: &str) {
        if let Some(drive) = agent.drives.get(drive_type) {
            let bar_width = 30;
            let filled = (drive.value * bar_width as f32) as usize;
            let empty = bar_width - filled;

            let bar_char = if drive.is_active() { '█' } else { '▓' };
            let bar = format!("{}{}",
                bar_char.to_string().repeat(filled),
                '░'.to_string().repeat(empty)
            );

            println!("│   {} [{}] {:.2}", label, bar, drive.value);
        }
    }

    /// Render the legend
    fn render_legend(&self) {
        println!("┌─ Legend ──────────────────────────────────────────────────────────────┐");
        println!("│  A-E: Agents  |  .: Empty space  |  ▓: Active drive  |  ░: Inactive    │");
        println!("│  Drives accumulate over time and trigger behavior tree execution       │");
        println!("│  Behavior trees learn through weight reinforcement (±10% per action)   │");
        println!("└───────────────────────────────────────────────────────────────────────┘");
    }

    /// Render a compact single-line status
    pub fn render_compact(&self, population: &Population, tick: u32) {
        print!("\rTick {:5} | ", tick);

        // Show average drives
        let mut total_hunger = 0.0;
        let mut total_rest = 0.0;
        let count = population.agents.len() as f32;

        for agent in &population.agents {
            if let Some(h) = agent.drives.get(DriveType::Hunger) {
                total_hunger += h.value;
            }
            if let Some(r) = agent.drives.get(DriveType::Rest) {
                total_rest += r.value;
            }
        }

        print!("Avg Hunger: {:.2} | Avg Rest: {:.2} | Agents: {} ",
            total_hunger / count,
            total_rest / count,
            population.agents.len()
        );
    }
}

impl Default for AsciiRenderer {
    fn default() -> Self {
        Self::new(80, 40)
    }
}
