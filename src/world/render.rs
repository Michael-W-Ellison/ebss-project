// src/world/render.rs
//! ASCII rendering system for world visualization.

use crate::world::{World, Position};
use crate::agents::Population;

/// ASCII renderer for the world
pub struct AsciiRenderer {
    pub use_color: bool,
    pub show_grid: bool,
}

impl AsciiRenderer {
    pub fn new() -> Self {
        Self {
            use_color: true,
            show_grid: false,
        }
    }

    /// Render the entire world and population
    pub fn render(&self, world: &World, population: &Population, viewport: Option<ViewPort>) -> String {
        let mut output = String::new();

        // Determine rendering area
        let (start_x, start_y, end_x, end_y) = if let Some(vp) = viewport {
            (vp.x, vp.y, (vp.x + vp.width).min(world.grid.width), (vp.y + vp.height).min(world.grid.height))
        } else {
            (0, 0, world.grid.width.min(80), world.grid.height.min(40)) // Limit to terminal size
        };

        // Top border
        output.push_str(&"=".repeat(end_x - start_x + 2));
        output.push('\n');

        // Render each row
        for y in start_y..end_y {
            output.push('|');

            for x in start_x..end_x {
                let pos = Position::new(x as i32, y as i32);
                let char_to_render = self.get_char_at(world, population, pos);

                if self.use_color {
                    let color = self.get_color_at(world, pos);
                    output.push_str(color);
                    output.push(char_to_render);
                    output.push_str("\x1b[0m"); // Reset color
                } else {
                    output.push(char_to_render);
                }
            }

            output.push('|');
            output.push('\n');
        }

        // Bottom border
        output.push_str(&"=".repeat(end_x - start_x + 2));
        output.push('\n');

        output
    }

    /// Get character to render at position
    fn get_char_at(&self, world: &World, population: &Population, pos: Position) -> char {
        // Check for agents first (highest priority)
        if population.agents.iter().any(|a| a.state.position == (pos.x, pos.y, 0)) {
            return '@';
        }

        // Check for buildings
        if let Some(building) = world.get_building_at(&pos) {
            return building.building_type.ascii_char();
        }

        // Check for resources
        if let Some(resource) = world.get_resource_at(&pos) {
            return resource.resource_type.ascii_char();
        }

        // Show terrain
        if let Some(tile) = world.grid.get_tile(&pos) {
            return tile.terrain.ascii_char();
        }

        ' '
    }

    /// Get color code for position
    fn get_color_at(&self, world: &World, pos: Position) -> &'static str {
        // Check for agents
        // (Would need population access - skipping for now)

        // Check for buildings
        if let Some(building) = world.get_building_at(&pos) {
            return building.building_type.color_code();
        }

        // Check for resources
        if let Some(resource) = world.get_resource_at(&pos) {
            return resource.resource_type.color_code();
        }

        // Show terrain color
        if let Some(tile) = world.grid.get_tile(&pos) {
            return tile.terrain.color_code();
        }

        "\x1b[0m" // Default/reset
    }

    /// Render statistics panel
    pub fn render_stats(&self, world: &World, population: &Population) -> String {
        let mut output = String::new();
        let stats = world.stats();

        output.push_str(&format!("\n=== WORLD STATISTICS (Tick {}) ===\n", world.tick));

        // Population
        output.push_str(&format!("Population: {} agents\n", population.agents.len()));
        output.push_str(&format!("  Avg Happiness: {:.2}\n", population.stats.average_happiness));

        // Resources available in world
        output.push_str(&format!("\nResources in World: {}\n", stats.total_resources));
        output.push_str(&format!("  Wood nodes: {} units\n", stats.wood_available));
        output.push_str(&format!("  Stone nodes: {} units\n", stats.stone_available));
        output.push_str(&format!("  Iron nodes: {} units\n", stats.iron_available));
        output.push_str(&format!("  Food nodes: {} units\n", stats.food_available));

        // Resources in storehouse
        output.push_str("\nStorehouse Inventory:\n");
        output.push_str(&format!("  Wood: {}\n", stats.wood_stored));
        output.push_str(&format!("  Stone: {}\n", stats.stone_stored));
        output.push_str(&format!("  Iron: {}\n", stats.iron_stored));
        output.push_str(&format!("  Food: {}\n", stats.food_stored));

        // Buildings
        output.push_str(&format!("\nBuildings: {}\n", stats.total_buildings));
        if stats.longhouses > 0 {
            output.push_str(&format!("  Longhouses: {}\n", stats.longhouses));
        }
        if stats.small_houses > 0 {
            output.push_str(&format!("  Small Houses: {}\n", stats.small_houses));
        }
        if stats.medium_houses > 0 {
            output.push_str(&format!("  Medium Houses: {}\n", stats.medium_houses));
        }
        if stats.large_houses > 0 {
            output.push_str(&format!("  Large Houses: {}\n", stats.large_houses));
        }
        if stats.storehouses > 0 {
            output.push_str(&format!("  Storehouses: {}\n", stats.storehouses));
        }
        if stats.workshops > 0 {
            output.push_str(&format!("  Workshops: {}\n", stats.workshops));
        }
        if stats.smithies > 0 {
            output.push_str(&format!("  Smithies: {}\n", stats.smithies));
        }
        if stats.farms > 0 {
            output.push_str(&format!("  Farms: {}\n", stats.farms));
        }

        output.push('\n');
        output
    }

    /// Render legend
    pub fn render_legend(&self) -> String {
        let mut output = String::new();

        output.push_str("\n=== LEGEND ===\n");
        output.push_str("@ = Agent\n");
        output.push_str("\nBuildings:\n");
        output.push_str("  L = Longhouse\n");
        output.push_str("  h = Small House\n");
        output.push_str("  H = Medium House\n");
        output.push_str("  # = Large House\n");
        output.push_str("  S = Storehouse\n");
        output.push_str("  W = Workshop\n");
        output.push_str("  M = Smithy\n");
        output.push_str("  F = Farm\n");

        output.push_str("\nResources:\n");
        output.push_str("  t = Wood (trees)\n");
        output.push_str("  s = Stone\n");
        output.push_str("  i = Iron\n");
        output.push_str("  f = Food\n");

        output.push_str("\nTerrain:\n");
        output.push_str("  . = Plains\n");
        output.push_str("  T = Forest\n");
        output.push_str("  ^ = Mountain\n");
        output.push_str("  ~ = Water\n");

        output.push('\n');
        output
    }

    /// Clear terminal screen (ANSI escape codes)
    pub fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H");
    }

    /// Render full frame (clear + world + stats)
    pub fn render_frame(&self, world: &World, population: &Population, viewport: Option<ViewPort>) -> String {
        let mut output = String::new();

        // Clear screen first
        output.push_str("\x1b[2J\x1b[H");

        // Render world
        output.push_str(&self.render(world, population, viewport));

        // Render stats
        output.push_str(&self.render_stats(world, population));

        output
    }
}

impl Default for AsciiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Viewport for rendering a portion of the world
#[derive(Debug, Clone, Copy)]
pub struct ViewPort {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

impl ViewPort {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Self {
        Self { x, y, width, height }
    }

    /// Create viewport centered on a position
    pub fn centered_on(pos: &Position, width: usize, height: usize, world_width: usize, world_height: usize) -> Self {
        let x = (pos.x as usize).saturating_sub(width / 2);
        let y = (pos.y as usize).saturating_sub(height / 2);

        let x = x.min(world_width.saturating_sub(width));
        let y = y.min(world_height.saturating_sub(height));

        Self { x, y, width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldConfig;

    #[test]
    fn test_renderer_creation() {
        let renderer = AsciiRenderer::new();
        assert!(renderer.use_color);
        assert!(!renderer.show_grid);
    }

    #[test]
    fn test_render_basic() {
        let world = World::new(WorldConfig {
            size: (10, 10),
            initial_resources: crate::world::ResourceConfig {
                wood_nodes: 2,
                stone_nodes: 2,
                iron_nodes: 1,
                food_nodes: 2,
                ..Default::default()
            },
        });
        let population = Population::new();

        let renderer = AsciiRenderer::new();
        let output = renderer.render(&world, &population, None);

        // Should contain borders
        assert!(output.contains('='));
        assert!(output.contains('|'));
    }

    #[test]
    fn test_render_stats() {
        let world = World::new(WorldConfig::default());
        let population = Population::new();

        let renderer = AsciiRenderer::new();
        let stats = renderer.render_stats(&world, &population);

        assert!(stats.contains("WORLD STATISTICS"));
        assert!(stats.contains("Population"));
        assert!(stats.contains("Storehouse"));
    }

    #[test]
    fn test_viewport() {
        let vp = ViewPort::new(5, 5, 20, 20);
        assert_eq!(vp.x, 5);
        assert_eq!(vp.y, 5);
        assert_eq!(vp.width, 20);
        assert_eq!(vp.height, 20);

        let centered = ViewPort::centered_on(&Position::new(25, 25), 20, 20, 50, 50);
        assert_eq!(centered.width, 20);
        assert_eq!(centered.height, 20);
    }
}
