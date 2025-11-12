# EBSS Visualization & Inspection System

Comprehensive system for pausing, inspecting, and visualizing the simulation in real-time.

## Overview

The visualization system provides:
- **Simulation Control**: Pause, play, and step through simulation ticks
- **Agent Inspection**: View detailed agent stats, drives, and behaviors
- **Terrain Inspection**: Examine materials and properties at any position
- **Selection System**: Select and highlight agents or terrain elements
- **Data Caching**: Fast access to frequently viewed information

## Core Components

### 1. Simulation Controller (`SimulationController`)

Controls simulation execution and provides inspection access.

```rust
use ebss::prelude::*;

// Create controller
let world = World::new(GridConfig::default());
let population = Population::new();
let mut controller = SimulationController::new(world, population);

// Control simulation
controller.play();           // Start running
controller.pause();           // Pause simulation
controller.toggle_pause();    // Toggle between paused/running
controller.step();            // Execute one tick
controller.set_tick_rate(20.0); // Set speed (ticks/second)

// Update in game loop
controller.update(delta_time);

// Access simulation data
let agents = controller.get_population();
let world = controller.get_world();
```

### 2. Inspector (`Inspector`)

Manages selection and provides detailed data views.

```rust
use ebss::analytics::{Inspector, AgentInspectorData};

let mut inspector = Inspector::new();

// Select an agent
inspector.select_agent(agent_id);

// Select terrain
inspector.select_terrain((x, y, z));

// Clear selection
inspector.clear_selection();

// Check current selection
match inspector.get_selection() {
    Selection::Agent(id) => { /* Handle agent selection */ },
    Selection::Terrain(pos) => { /* Handle terrain selection */ },
    Selection::None => { /* No selection */ },
}
```

### 3. Agent Inspector Data (`AgentInspectorData`)

Comprehensive agent information for display.

```rust
use ebss::analytics::AgentInspectorData;

// Get inspector data from agent
let data = AgentInspectorData::from_agent(&agent);

// Access basic info
println!("ID: {}", data.id);
println!("Position: {:?}", data.position);
println!("Health: {:.1}", data.health);

// View drives
for drive in &data.drives {
    println!("{}: {:.2} (urgency: {:.2})",
        drive.name, drive.value, drive.urgency);
}

// Get drives sorted by urgency
let urgent_drives = data.drives_by_urgency();

// Get only active drives
let active = data.active_drives();

// Find most urgent drive
if let Some(drive_type) = data.most_urgent_drive {
    println!("Most urgent: {:?}", drive_type);
}
```

### 4. Drive Inspector Data (`DriveInspectorData`)

Detailed drive state information.

```rust
pub struct DriveInspectorData {
    pub drive_type: DriveType,
    pub name: String,
    pub value: f32,           // Current value (0.0-1.0)
    pub threshold: f32,        // Activation threshold
    pub weight: f32,           // Personality weight
    pub urgency: f32,          // value * weight
    pub is_active: bool,       // Above threshold
    pub satisfaction: String,  // How to satisfy
}
```

## Running the Demo

```bash
cargo run --example inspector_demo
```

This demonstrates:
- Simulation pause/play/step controls
- Agent data inspection
- Drive state visualization
- Selection system
- Data caching

## Integration with GUI Frameworks

### Option 1: egui (Immediate Mode GUI)

```toml
[dependencies]
egui = "0.30"
eframe = "0.30"
```

```rust
use eframe::egui;
use ebss::prelude::*;

struct SimulatorApp {
    controller: SimulationController,
    inspector: Inspector,
}

impl eframe::App for SimulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Control panel
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.controller.is_running() { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    self.controller.toggle_pause();
                }
                if ui.button("⏭ Step").clicked() {
                    self.controller.step();
                }
                ui.label(format!("Tick: {}", self.controller.current_tick));
            });
        });

        // Agent inspector panel
        egui::SidePanel::right("inspector").show(ctx, |ui| {
            ui.heading("Inspector");
            if let Selection::Agent(id) = self.inspector.get_selection() {
                if let Some(data) = self.inspector.get_cached_agent_data(*id) {
                    ui.label(format!("Agent: {}", data.id));
                    ui.label(format!("Health: {:.1}", data.health));

                    ui.separator();
                    ui.label("Drives:");
                    for drive in data.drives_by_urgency().iter().take(5) {
                        ui.horizontal(|ui| {
                            ui.label(&drive.name);
                            ui.add(egui::ProgressBar::new(drive.value));
                        });
                    }
                }
            }
        });

        // Main viewport
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render world and agents here
        });

        // Update simulation
        if self.controller.is_running() {
            let dt = ctx.input(|i| i.stable_dt);
            self.controller.update(dt);
            ctx.request_repaint();
        }
    }
}
```

### Option 2: Bevy (Game Engine)

```toml
[dependencies]
bevy = "0.15"
```

```rust
use bevy::prelude::*;
use ebss::prelude::*;

#[derive(Resource)]
struct SimulatorState {
    controller: SimulationController,
    inspector: Inspector,
}

fn update_simulation(
    mut state: ResMut<SimulatorState>,
    time: Res<Time>,
) {
    if state.controller.is_running() {
        state.controller.update(time.delta_secs());
    }
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SimulatorState>,
) {
    if keys.just_pressed(KeyCode::Space) {
        state.controller.toggle_pause();
    }
    if keys.just_pressed(KeyCode::KeyS) {
        state.controller.step();
    }
}
```

## UI Features to Implement

### Agent Selection & Display
- **Click to select**: Mouse click on agent sprite/model
- **Highlight selected**: Border, glow, or color change
- **Info panel**: Show agent data in side panel
- **Drive bars**: Visual progress bars for each drive
- **Behavior tree**: Hierarchical view of current behaviors

### Terrain Inspection
- **Hover tooltip**: Show material name on hover
- **Click for details**: Full material properties in dialog
- **Height map**: Visualize terrain elevation
- **Material overlay**: Toggle material type colors

### Simulation Controls
- **Play/Pause button**: Toggle simulation
- **Step button**: Advance one tick
- **Speed slider**: Adjust tick rate
- **Tick counter**: Display current simulation tick
- **Time display**: Show in-simulation time

### Data Visualization
- **Drive graphs**: Line charts showing drive history
- **Population stats**: Bar charts of agent states
- **Resource maps**: Heat maps of material distribution
- **Agent trails**: Path visualization

## Performance Considerations

### Caching Strategy
```rust
// Update cache only when needed
if frame_count % 10 == 0 {
    inspector.update_cache(&controller.get_population().agents);
}

// Access cached data
if let Some(data) = inspector.get_cached_agent_data(agent_id) {
    // Use cached data for UI
}
```

### Selective Updates
```rust
// Only update visible agents
let visible_agents = get_agents_in_viewport();
for agent_id in visible_agents {
    if let Some(agent) = find_agent(agent_id) {
        let data = AgentInspectorData::from_agent(agent);
        inspector.cache_agent_data(agent_id, data);
    }
}
```

## Example Use Cases

### 1. Debug Mode
- Pause simulation at any point
- Inspect agent decision making
- Verify drive calculations
- Check behavior tree execution

### 2. Research Analysis
- Step through tick-by-tick
- Record agent state changes
- Analyze drive progression
- Study emergent behaviors

### 3. Player Interaction
- Click agents to view stats
- Monitor resource locations
- Track agent relationships
- Guide agent decisions

## API Reference

See full API documentation:
- `src/analytics/simulation_controller.rs` - Simulation control
- `src/analytics/inspector.rs` - Inspection system
- `examples/inspector_demo.rs` - Complete working example

## Future Enhancements

- [ ] Relationship graph visualization
- [ ] Memory inspection (spatial, social, recipe)
- [ ] Behavior tree execution viewer
- [ ] Agent comparison tool
- [ ] Time travel (rewind simulation)
- [ ] Save/load inspection states
- [ ] Screenshot/video capture
- [ ] Performance profiling overlay
