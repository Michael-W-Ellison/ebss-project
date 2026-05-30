// src/visualization/mod.rs
//! ASCII visualization for simulation state.
//!
//! This module provides comprehensive ASCII-based visualization including:
//! - Real-time agent status displays
//! - Population statistics and trends
//! - Drive level visualization with color support
//! - World grid rendering
//! - Historical data tracking
//! - Multiple rendering modes
//! - Streaming output for external tools

pub mod streaming;

pub use streaming::{
    StreamFormat, StreamOutput, ConsoleOutput, FileOutput, BufferOutput, MultiOutput,
    StreamEvent, StreamFormatter, StreamingVisualizer, StreamConfig,
    DisplayWidget, WidgetData, TextWidget, ProgressWidget, WidgetDashboard,
};

use crate::agents::{Agent, Population};
use crate::core::DriveType;
use std::collections::VecDeque;

/// ANSI color codes for terminal output
pub mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    // Foreground colors
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";

    // Bright colors
    pub const BRIGHT_RED: &str = "\x1b[91m";
    pub const BRIGHT_GREEN: &str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &str = "\x1b[93m";
    pub const BRIGHT_BLUE: &str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
    pub const BRIGHT_CYAN: &str = "\x1b[96m";

    // Background colors
    pub const BG_RED: &str = "\x1b[41m";
    pub const BG_GREEN: &str = "\x1b[42m";
    pub const BG_YELLOW: &str = "\x1b[43m";
    pub const BG_BLUE: &str = "\x1b[44m";
}

/// Rendering mode for the ASCII renderer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Full detailed view with all panels
    Full,
    /// Compact view focusing on essential info
    Compact,
    /// Dashboard view with statistics and trends
    Dashboard,
    /// World-focused view with larger map
    WorldFocus,
    /// Agent-focused view with detailed agent info
    AgentFocus,
}

/// Historical data point for tracking trends
#[derive(Debug, Clone)]
pub struct HistoryPoint {
    pub tick: u32,
    pub population_size: usize,
    pub average_health: f32,
    pub average_energy: f32,
    pub average_hunger: f32,
    pub average_happiness: f32,
    pub births: u32,
    pub deaths: u32,
}

/// Statistics tracker for population metrics
#[derive(Debug, Clone)]
pub struct PopulationStats {
    pub total_agents: usize,
    pub alive_agents: usize,
    pub average_health: f32,
    pub average_energy: f32,
    pub average_age: f32,
    pub drive_averages: Vec<(DriveType, f32)>,
    pub most_common_drive: Option<DriveType>,
    pub births_this_session: u32,
    pub deaths_this_session: u32,
}

impl PopulationStats {
    /// Calculate statistics from a population
    pub fn from_population(population: &Population) -> Self {
        let agents = &population.agents;
        let count = agents.len();

        if count == 0 {
            return Self {
                total_agents: 0,
                alive_agents: 0,
                average_health: 0.0,
                average_energy: 0.0,
                average_age: 0.0,
                drive_averages: Vec::new(),
                most_common_drive: None,
                births_this_session: 0,
                deaths_this_session: 0,
            };
        }

        let count_f = count as f32;

        let average_health = agents.iter().map(|a| a.state.health).sum::<f32>() / count_f;
        let average_energy = agents.iter().map(|a| a.state.energy).sum::<f32>() / count_f;
        let average_age = agents.iter().map(|a| a.state.age as f32).sum::<f32>() / count_f;

        // Calculate drive averages
        let drive_types = [
            DriveType::Hunger, DriveType::Thirst, DriveType::Rest,
            DriveType::Safety, DriveType::Social, DriveType::Curiosity,
            DriveType::Shelter, DriveType::Industry, DriveType::Utility,
        ];

        let mut drive_averages = Vec::new();
        let mut max_drive: Option<(DriveType, f32)> = None;

        for dt in &drive_types {
            let sum: f32 = agents.iter()
                .filter_map(|a| a.drives.get(*dt))
                .map(|d| d.value)
                .sum();
            let avg = sum / count_f;
            drive_averages.push((*dt, avg));

            if max_drive.map(|(_, v)| avg > v).unwrap_or(true) {
                max_drive = Some((*dt, avg));
            }
        }

        Self {
            total_agents: count,
            alive_agents: agents.iter().filter(|a| a.state.health > 0.0).count(),
            average_health,
            average_energy,
            average_age,
            drive_averages,
            most_common_drive: max_drive.map(|(dt, _)| dt),
            births_this_session: 0,
            deaths_this_session: 0,
        }
    }
}

/// Configuration for the ASCII renderer
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub width: usize,
    pub height: usize,
    pub use_color: bool,
    pub use_unicode: bool,
    pub max_agents_display: usize,
    pub show_drives: Vec<DriveType>,
    pub world_grid_size: usize,
    pub history_length: usize,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 80,
            height: 40,
            use_color: true,
            use_unicode: true,
            max_agents_display: 8,
            show_drives: vec![
                DriveType::Hunger, DriveType::Thirst, DriveType::Rest,
                DriveType::Safety, DriveType::Social, DriveType::Curiosity,
            ],
            world_grid_size: 20,
            history_length: 100,
        }
    }
}

/// ASCII renderer for simulation visualization
pub struct AsciiRenderer {
    pub config: RenderConfig,
    pub mode: RenderMode,
    history: VecDeque<HistoryPoint>,
    event_log: VecDeque<String>,
    pub width: usize,
    pub height: usize,
}

impl AsciiRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            config: RenderConfig {
                width,
                height,
                ..Default::default()
            },
            mode: RenderMode::Full,
            history: VecDeque::new(),
            event_log: VecDeque::with_capacity(10),
            width,
            height,
        }
    }

    /// Create a new renderer with custom configuration
    pub fn with_config(config: RenderConfig) -> Self {
        let width = config.width;
        let height = config.height;
        Self {
            config,
            mode: RenderMode::Full,
            history: VecDeque::new(),
            event_log: VecDeque::with_capacity(10),
            width,
            height,
        }
    }

    /// Set the rendering mode
    pub fn set_mode(&mut self, mode: RenderMode) {
        self.mode = mode;
    }

    /// Log an event for display
    pub fn log_event(&mut self, event: String) {
        if self.event_log.len() >= 10 {
            self.event_log.pop_front();
        }
        self.event_log.push_back(event);
    }

    /// Record a history point
    pub fn record_history(&mut self, population: &Population, tick: u32) {
        let stats = PopulationStats::from_population(population);

        let hunger_avg = stats.drive_averages.iter()
            .find(|(dt, _)| *dt == DriveType::Hunger)
            .map(|(_, v)| *v)
            .unwrap_or(0.0);

        let point = HistoryPoint {
            tick,
            population_size: stats.alive_agents,
            average_health: stats.average_health,
            average_energy: stats.average_energy,
            average_hunger: hunger_avg,
            average_happiness: 100.0 - hunger_avg * 100.0, // Simple happiness metric
            births: 0,
            deaths: 0,
        };

        if self.history.len() >= self.config.history_length {
            self.history.pop_front();
        }
        self.history.push_back(point);
    }

    /// Render the complete simulation state
    pub fn render(&self, population: &Population, tick: u32) {
        match self.mode {
            RenderMode::Full => self.render_full(population, tick),
            RenderMode::Compact => { self.render_compact_line(population, tick); },
            RenderMode::Dashboard => self.render_dashboard(population, tick),
            RenderMode::WorldFocus => self.render_world_focus(population, tick),
            RenderMode::AgentFocus => self.render_agent_focus(population, tick),
        }
    }

    /// Full detailed render
    fn render_full(&self, population: &Population, tick: u32) {
        self.clear_screen();
        self.render_header(tick, population.agents.len());
        self.render_world(population);
        self.render_agent_status(population);
        self.render_statistics(population);
        self.render_legend();
    }

    /// Dashboard render with statistics focus
    fn render_dashboard(&self, population: &Population, tick: u32) {
        self.clear_screen();
        self.render_header(tick, population.agents.len());
        self.render_statistics(population);
        self.render_trend_chart();
        self.render_drive_overview(population);
        self.render_event_log();
    }

    /// World-focused render with large map
    fn render_world_focus(&self, population: &Population, tick: u32) {
        self.clear_screen();
        self.render_header(tick, population.agents.len());
        self.render_large_world(population);
        self.render_compact_stats(population);
    }

    /// Agent-focused render with detailed agent info
    fn render_agent_focus(&self, population: &Population, tick: u32) {
        self.clear_screen();
        self.render_header(tick, population.agents.len());
        self.render_detailed_agents(population);
    }

    /// Clear the terminal screen
    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    /// Render the header with tick and population info
    fn render_header(&self, tick: u32, population_size: usize) {
        let color = if self.config.use_color { colors::BRIGHT_CYAN } else { "" };
        let reset = if self.config.use_color { colors::RESET } else { "" };
        let bold = if self.config.use_color { colors::BOLD } else { "" };

        println!("{}╔══════════════════════════════════════════════════════════════════════════╗{}", color, reset);
        println!("{}║{}  EBSS - Emergent Behavior Society Simulator                              {}║{}", color, bold, reset, reset);
        println!("{}║  Tick: {:6}  │  Population: {:3}  │  Mode: {:12}              ║{}",
            color, tick, population_size, format!("{:?}", self.mode), reset);
        println!("{}╚══════════════════════════════════════════════════════════════════════════╝{}", color, reset);
        println!();
    }

    /// Render the world grid with agent positions
    fn render_world(&self, population: &Population) {
        let grid_size = self.config.world_grid_size;
        println!("┌─ World View ({}x{}) ─────────────────────────────────────────────────┐",
            grid_size, grid_size);

        let mut grid = vec![vec!['.'; grid_size]; grid_size];

        // Place agents on the grid
        for (idx, agent) in population.agents.iter().enumerate() {
            let x = ((agent.state.position.0.abs() % grid_size as i32) as usize).min(grid_size - 1);
            let y = ((agent.state.position.1.abs() % grid_size as i32) as usize).min(grid_size - 1);
            let symbol = Self::agent_symbol(idx);
            grid[y][x] = symbol;
        }

        // Render the grid with optional color
        for row in grid {
            print!("│ ");
            for cell in row {
                if self.config.use_color && cell != '.' {
                    print!("{}{}{} ", colors::BRIGHT_GREEN, cell, colors::RESET);
                } else {
                    print!("{} ", cell);
                }
            }
            println!("│");
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Render a larger world view
    fn render_large_world(&self, population: &Population) {
        let grid_size = 40;
        println!("┌─ World View ({}x{}) ─────────────────────────────────────────────────┐",
            grid_size, grid_size);

        let mut grid = vec![vec!['.'; grid_size]; grid_size];

        for (idx, agent) in population.agents.iter().enumerate() {
            let x = ((agent.state.position.0.abs() % grid_size as i32) as usize).min(grid_size - 1);
            let y = ((agent.state.position.1.abs() % grid_size as i32) as usize).min(grid_size - 1);
            grid[y][x] = Self::agent_symbol(idx);
        }

        for (y, row) in grid.iter().enumerate() {
            if y % 2 == 0 { // Show every other row for compactness
                print!("│");
                for cell in row {
                    if self.config.use_color && *cell != '.' {
                        print!("{}{}{}", colors::BRIGHT_GREEN, cell, colors::RESET);
                    } else {
                        print!("{}", cell);
                    }
                }
                println!("│");
            }
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Get symbol for agent index
    fn agent_symbol(idx: usize) -> char {
        const SYMBOLS: &[char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
                                   'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
                                   'U', 'V', 'W', 'X', 'Y', 'Z'];
        if idx < SYMBOLS.len() {
            SYMBOLS[idx]
        } else {
            '*'
        }
    }

    /// Render agent status panels
    fn render_agent_status(&self, population: &Population) {
        let max_display = self.config.max_agents_display;
        let total = population.agents.len();

        println!("┌─ Agent Status ({}/{} shown) ─────────────────────────────────────────────┐",
            max_display.min(total), total);

        for (idx, agent) in population.agents.iter().enumerate().take(max_display) {
            self.render_single_agent(agent, idx);
        }

        if total > max_display {
            println!("│  ... and {} more agents", total - max_display);
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Render detailed agent information
    fn render_detailed_agents(&self, population: &Population) {
        println!("┌─ Detailed Agent Information ────────────────────────────────────────────┐");

        for (idx, agent) in population.agents.iter().enumerate().take(4) {
            let symbol = Self::agent_symbol(idx);

            println!("│");
            println!("│ {}Agent {}{}: ID {}...",
                colors::BOLD, symbol, colors::RESET,
                &agent.id.to_string()[0..8]);
            println!("│   Position: ({}, {}, {})",
                agent.state.position.0, agent.state.position.1, agent.state.position.2);
            println!("│   Health: {:5.1}%  │  Energy: {:5.1}%  │  Age: {} ticks",
                agent.state.health, agent.state.energy, agent.state.age);
            println!("│   Life Stage: {:?}  │  Ticks without food: {}",
                agent.state.life_stage, agent.state.ticks_without_food);

            // Show all drives
            println!("│   Drives:");
            let drive_types = [
                DriveType::Hunger, DriveType::Thirst, DriveType::Rest,
                DriveType::Safety, DriveType::Social, DriveType::Curiosity,
            ];
            for dt in &drive_types {
                if let Some(drive) = agent.drives.get(*dt) {
                    let bar = self.make_bar(drive.value, 20);
                    let status = if drive.is_active() { "ACTIVE" } else { "      " };
                    println!("│     {:12?}: {} {:5.2} {}", dt, bar, drive.value, status);
                }
            }

            // Show inventory summary
            let item_count = agent.inventory.get_all_items().len();
            println!("│   Inventory: {} item types", item_count);

            // Show behavior tree stats
            if let Some(tree) = agent.behavior_trees.first() {
                println!("│   Learning: {} executions, {:.1}% success",
                    tree.total_executions, tree.success_rate() * 100.0);
            }
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Render a single agent's status
    fn render_single_agent(&self, agent: &Agent, idx: usize) {
        let symbol = Self::agent_symbol(idx);
        let urgent_drive = agent.drives.most_urgent();
        let urgent_name = urgent_drive.map(|d| format!("{:?}", d.drive_type)).unwrap_or("None".to_string());
        let urgent_value = urgent_drive.map(|d| d.value).unwrap_or(0.0);

        // Health color
        let health_color = if self.config.use_color {
            if agent.state.health > 70.0 { colors::BRIGHT_GREEN }
            else if agent.state.health > 30.0 { colors::YELLOW }
            else { colors::BRIGHT_RED }
        } else { "" };

        println!("│");
        println!("│ Agent {} [Health: {}{}%{} | Energy: {:3.0}%]  Current Drive: {} ({:.2})",
            symbol,
            health_color,
            agent.state.health as i32,
            if self.config.use_color { colors::RESET } else { "" },
            agent.state.energy,
            urgent_name,
            urgent_value
        );

        // Render configured drives
        for drive_type in &self.config.show_drives {
            self.render_drive_bar(agent, *drive_type);
        }

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
    fn render_drive_bar(&self, agent: &Agent, drive_type: DriveType) {
        if let Some(drive) = agent.drives.get(drive_type) {
            let label = format!("{:10?}", drive_type);
            let bar = self.make_colored_bar(drive.value, 25, drive.is_active());
            println!("│   {} {} {:.2}", label, bar, drive.value);
        }
    }

    /// Create a progress bar string
    fn make_bar(&self, value: f32, width: usize) -> String {
        let filled = (value * width as f32) as usize;
        let empty = width.saturating_sub(filled);

        if self.config.use_unicode {
            format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
        } else {
            format!("[{}{}]", "#".repeat(filled), "-".repeat(empty))
        }
    }

    /// Create a colored progress bar
    fn make_colored_bar(&self, value: f32, width: usize, is_active: bool) -> String {
        let filled = (value * width as f32) as usize;
        let empty = width.saturating_sub(filled);

        let (fill_char, empty_char) = if self.config.use_unicode {
            if is_active { ('█', '░') } else { ('▓', '░') }
        } else {
            if is_active { ('#', '-') } else { ('=', '-') }
        };

        let color = if self.config.use_color {
            if value > 0.7 { colors::BRIGHT_RED }
            else if value > 0.4 { colors::YELLOW }
            else { colors::BRIGHT_GREEN }
        } else { "" };

        let reset = if self.config.use_color { colors::RESET } else { "" };

        format!("[{}{}{}{}]",
            color,
            fill_char.to_string().repeat(filled),
            reset,
            empty_char.to_string().repeat(empty))
    }

    /// Render population statistics
    fn render_statistics(&self, population: &Population) {
        let stats = PopulationStats::from_population(population);

        println!("┌─ Population Statistics ──────────────────────────────────────────────────┐");
        println!("│  Total Agents: {:4}  │  Alive: {:4}  │  Avg Age: {:6.0} ticks",
            stats.total_agents, stats.alive_agents, stats.average_age);
        println!("│  Avg Health: {:5.1}%  │  Avg Energy: {:5.1}%",
            stats.average_health, stats.average_energy);

        if let Some(drive) = stats.most_common_drive {
            println!("│  Most Active Drive: {:?}", drive);
        }

        println!("│");
        println!("│  Drive Averages:");
        for (dt, avg) in &stats.drive_averages {
            let bar = self.make_bar(*avg, 15);
            println!("│    {:12?}: {} {:.2}", dt, bar, avg);
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
        println!();
    }

    /// Render compact statistics
    fn render_compact_stats(&self, population: &Population) {
        let stats = PopulationStats::from_population(population);
        println!("│ Pop: {} │ Health: {:.0}% │ Energy: {:.0}% │ Urgent: {:?}",
            stats.alive_agents, stats.average_health, stats.average_energy,
            stats.most_common_drive.unwrap_or(DriveType::Hunger));
    }

    /// Render drive overview for all agents
    fn render_drive_overview(&self, population: &Population) {
        println!("┌─ Drive Overview ─────────────────────────────────────────────────────────┐");

        let drive_types = [
            DriveType::Hunger, DriveType::Thirst, DriveType::Rest,
            DriveType::Safety, DriveType::Social, DriveType::Curiosity,
        ];

        // Header
        print!("│ Agent │");
        for dt in &drive_types {
            print!(" {:7?} │", dt);
        }
        println!();
        println!("│───────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────│");

        // Each agent
        for (idx, agent) in population.agents.iter().enumerate().take(10) {
            print!("│   {}   │", Self::agent_symbol(idx));
            for dt in &drive_types {
                if let Some(drive) = agent.drives.get(*dt) {
                    let color = if self.config.use_color {
                        if drive.value > 0.7 { colors::BRIGHT_RED }
                        else if drive.value > 0.4 { colors::YELLOW }
                        else { colors::BRIGHT_GREEN }
                    } else { "" };
                    print!(" {}{:7.2}{} │", color, drive.value,
                        if self.config.use_color { colors::RESET } else { "" });
                } else {
                    print!("    -    │");
                }
            }
            println!();
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Render trend chart from history
    fn render_trend_chart(&self) {
        if self.history.is_empty() {
            return;
        }

        println!("┌─ Population Trend (last {} ticks) ─────────────────────────────────────┐",
            self.history.len());

        // Simple ASCII trend line
        let height = 5;
        let width = 60.min(self.history.len());

        if width > 0 {
            let recent: Vec<_> = self.history.iter().rev().take(width).collect();
            let max_pop = recent.iter().map(|h| h.population_size).max().unwrap_or(1);

            for row in (0..height).rev() {
                print!("│ ");
                for point in recent.iter().rev() {
                    let normalized = (point.population_size as f32 / max_pop as f32 * height as f32) as usize;
                    if normalized > row {
                        print!("█");
                    } else {
                        print!(" ");
                    }
                }
                println!(" │");
            }
            println!("│ {:>width$} │", format!("Pop: 0-{}", max_pop), width = width);
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Render event log
    fn render_event_log(&self) {
        println!("┌─ Recent Events ──────────────────────────────────────────────────────────┐");

        if self.event_log.is_empty() {
            println!("│  (No events recorded)");
        } else {
            for event in &self.event_log {
                println!("│  • {}", event);
            }
        }

        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Render the legend
    fn render_legend(&self) {
        println!("┌─ Legend ────────────────────────────────────────────────────────────────┐");
        println!("│  A-Z: Agents  │  .: Empty space  │  █: Active drive  │  ░: Inactive   │");
        println!("│  Drives accumulate over time and trigger behavior tree execution       │");
        println!("│  Colors: {}Green{}=Low │ {}Yellow{}=Medium │ {}Red{}=High (urgent)              │",
            if self.config.use_color { colors::BRIGHT_GREEN } else { "" },
            if self.config.use_color { colors::RESET } else { "" },
            if self.config.use_color { colors::YELLOW } else { "" },
            if self.config.use_color { colors::RESET } else { "" },
            if self.config.use_color { colors::BRIGHT_RED } else { "" },
            if self.config.use_color { colors::RESET } else { "" });
        println!("│  Modes: Full | Compact | Dashboard | WorldFocus | AgentFocus           │");
        println!("└───────────────────────────────────────────────────────────────────────────┘");
    }

    /// Render a compact single-line status (non-clearing) - internal
    fn render_compact_line(&self, population: &Population, tick: u32) {
        let stats = PopulationStats::from_population(population);

        let color = if self.config.use_color { colors::CYAN } else { "" };
        let reset = if self.config.use_color { colors::RESET } else { "" };

        print!("\r{}Tick {:5}{} │ Pop: {:3} │ Health: {:5.1}% │ Energy: {:5.1}% │ ",
            color, tick, reset,
            stats.alive_agents,
            stats.average_health,
            stats.average_energy);

        // Show top 3 drives
        let mut sorted_drives = stats.drive_averages.clone();
        sorted_drives.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (dt, val) in sorted_drives.iter().take(3) {
            print!("{:?}:{:.2} ", dt, val);
        }

        print!("        "); // Clear trailing characters
    }

    /// Render a compact single-line status (for backwards compatibility)
    pub fn render_compact(&self, population: &Population, tick: u32) {
        self.render_compact_line(population, tick);
    }
}

impl Default for AsciiRenderer {
    fn default() -> Self {
        Self::with_config(RenderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = AsciiRenderer::new(80, 40);
        assert_eq!(renderer.width, 80);
        assert_eq!(renderer.height, 40);
    }

    #[test]
    fn test_render_mode() {
        let mut renderer = AsciiRenderer::default();
        assert_eq!(renderer.mode, RenderMode::Full);

        renderer.set_mode(RenderMode::Dashboard);
        assert_eq!(renderer.mode, RenderMode::Dashboard);
    }

    #[test]
    fn test_population_stats() {
        let population = Population::new();
        let stats = PopulationStats::from_population(&population);

        assert_eq!(stats.total_agents, 0);
        assert_eq!(stats.average_health, 0.0);
    }

    #[test]
    fn test_event_logging() {
        let mut renderer = AsciiRenderer::default();

        for i in 0..15 {
            renderer.log_event(format!("Event {}", i));
        }

        // Should only keep last 10 events
        assert_eq!(renderer.event_log.len(), 10);
    }

    #[test]
    fn test_history_recording() {
        let mut renderer = AsciiRenderer::default();
        let population = Population::new();

        for tick in 0..150 {
            renderer.record_history(&population, tick);
        }

        // Should cap at history_length (default 100)
        assert!(renderer.history.len() <= renderer.config.history_length);
    }

    #[test]
    fn test_agent_symbol() {
        assert_eq!(AsciiRenderer::agent_symbol(0), 'A');
        assert_eq!(AsciiRenderer::agent_symbol(25), 'Z');
        assert_eq!(AsciiRenderer::agent_symbol(26), '*');
    }

    #[test]
    fn test_make_bar() {
        let renderer = AsciiRenderer::default();

        let bar = renderer.make_bar(0.5, 10);
        assert!(bar.contains('['));
        assert!(bar.contains(']'));
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();

        assert_eq!(config.width, 80);
        assert_eq!(config.height, 40);
        assert!(config.use_color);
        assert!(config.use_unicode);
        assert_eq!(config.max_agents_display, 8);
    }
}
