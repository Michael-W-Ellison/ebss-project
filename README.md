# Emergent Behavior Society Simulator (EBSS)

A general-purpose AI platform for simulating societies of autonomous agents that learn and adapt through behavioral evolution.

## Overview

EBSS provides a modular framework where agents develop complex behaviors through:
- **Weighted Behavior Trees**: Learned decision-making patterns that evolve with experience
- **Drive-Based Motivation**: 14 core drives (hunger, thirst, safety, curiosity, social, etc.) creating dynamic priorities
- **Genetic Inheritance**: Offspring inherit successful behavioral patterns from parents
- **Memory Systems**: Agents remember locations, storage contents, and other agents
- **Observational Learning**: Young agents learn by following experienced agents
- **Modular Environments**: Plugin architecture for different world rules and game mechanics

Unlike game-specific implementations, EBSS is environment-agnostic, allowing researchers and developers to plug in different rule systems (Minecraft-style survival, Dwarf Fortress-inspired societies, medieval simulations, or entirely novel environments) while maintaining the same core AI architecture.

## Project Status

**Current state**: all four planned phases are implemented. A default
simulation runs a society that feeds itself, waters itself, shelters from the
weather, and reproduces over tens of thousands of ticks. Roughly 95,000 lines
across 181 source files, with 1,055 library tests.

Every build configuration compiles: default, `--features gui`,
`--features bevy_gui` and `--workspace`, with 1,092 tests across the workspace.
The work left is connecting rather than building — several analytics
components are libraries with no caller, and agents cannot yet see each other
or hear anything. See
[PROJECT_STATUS.txt](PROJECT_STATUS.txt) for measured detail and
[ISSUES_FOUND.md](ISSUES_FOUND.md) for the current defect list. The
[Software Design Document](EBSS_Software_Design_Document.docx) holds the
original specifications.

## Key Features

- ✅ Behavior Tree Learning: Agents build and evolve decision trees through experience
- ✅ Drive Architecture: 14 core drives create emergent behavior patterns
- ✅ Survival: hunger, thirst, nutrition, body temperature, exposure and shelter
- ✅ Genetic Inheritance: offspring inherit traits and behavior from parents
- ✅ Memory Systems: spatial and episodic memory with decay
- ✅ Social Learning: observation, imitation, gossip and shared knowledge
- ✅ Environment Abstraction: plugin interface, crafting, technology progression
- 🚧 Analytics: emergence detection, metrics and replay exist but the
  simulation loop does not drive them — examples do
- ✅ Fire and cooking: agents gather wood, light campfires and cook at them.
  Only meat, fish and grain are improved by a fire; anything else put over one
  is ruined, and so is anything cooked twice. Burning a batch gets rarer with
  practice
- ✅ Perception: sight is how agents find things — terrain, resources and
  buildings within 25 tiles, refreshed every tick, and the Blind trait takes
  it away. Smell is scaled to what a thing actually gives off: a berry on the
  bush carries about two tiles, water three, flesh six, food that has turned
  nine to twenty, and cooking the whole range
- 🚧 Agents cannot yet see each other or hear anything: those percept channels
  are built but unfed
- ✅ Clothing: agents gather flax, cotton and bark, make garments and wear
  them. A garment is worth what its material is worth and what the hand that
  made it could manage; wood goes into clothes only once a fire's worth is set
  aside
- ✅ Ecology: herds are held down by the predators that live off them and by
  the ground they graze. A predator that cannot find prey starves, widens what
  it will take, and turns on the people living beside it. A species wiped out of a
  world is slowly replaced by animals wandering in from off the map
- ✅ Hunting: agents go after animals for the skins and eat what comes with
  them. A hunter has to be within a spear's throw, an unarmed one leaves the
  dangerous animals alone, and a kill is butchered into meat, hides, leather
  and wool

Legend: ✅ Implemented and running | 🚧 Built but not fully connected | 📋 Not yet driven

## Project Structure

```
ebss-project/
├── src/
│   ├── core/           # Behavior trees, drives, learning algorithms
│   ├── agents/         # Agent state, lifecycle, decision-making
│   ├── environment/    # Environment abstraction and plugins
│   ├── world/          # Spatial simulation, resources, physics
│   └── analytics/      # Data logging, visualization, emergence detection
├── tests/              # Unit and integration tests
├── docs/               # Documentation and design documents
├── examples/           # Example simulations and tutorials
└── config/             # Environment configurations and presets
```

## Getting Started

### Prerequisites

- Rust 1.70+ (for core engine)
- Cargo (Rust package manager)
- (Optional) Lua 5.4+ (for environment plugins)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ebss-project.git
cd ebss-project

# Build the project
cargo build --release

# Run tests
cargo test

# Run example simulation
cargo run --example basic_survival
```

### Quick Start

```rust
use ebss::prelude::*;

fn main() {
    // Create a simple world
    let world = World::new(GridConfig {
        size: (100, 100, 10),
        chunk_size: 16,
    });

    // Add agents with basic drives
    let mut population = Population::new();
    for _ in 0..10 {
        population.spawn_agent(AgentConfig::default());
    }

    // Run simulation
    let mut sim = Simulation::new(world, population);
    sim.run_for_ticks(1000);

    // Analyze results
    println!("Emergent behaviors: {:?}", sim.analyze_behaviors());
}
```

## Development Roadmap

All four originally planned phases are implemented. Boxes below reflect what
the code actually does, verified by running it — not what was planned.

### Phase 1: Core Foundation ✅
- [x] Project structure and build system
- [x] Behavior tree implementation with weight-based learning and pruning
- [x] Core drive system (all 14 drives)
- [x] Grid-based world with terrain, resources and regeneration
- [x] Agent actions and learning
- [x] ASCII visualization

### Phase 2: Environment Abstraction ✅
- [x] Plugin architecture (`src/environment/plugin.rs`, registry)
- [x] Material property system
- [x] Template-based crafting, smelting and clothing recipes
- [x] Minecraft-style environment (`src/environment/minecraft_survival.rs`)
- [x] Tool effectiveness calculations
- [x] Bundled `plugins/minecraft_survival` crate, a worked example of the
      plugin interface — though it duplicates the in-tree module above

### Phase 3: Social Systems ✅
- [x] Reproduction, pregnancy, birth and nursing
- [x] Genetic and behavioral inheritance
- [x] Observational learning
- [x] Social memory, relationships, gossip and shared knowledge
- [x] All 14 drives implemented and acted on

### Phase 4: Analytics and Polish 🚧
- [x] Data logging and analysis (metrics, export to JSON/CSV)
- [x] Emergence detection
- [x] Save/load and autosave with checkpoint rotation
- [x] Interactive GUI (egui) alongside the ASCII renderer
- [ ] Analytics are not driven by the simulation loop — they run only when a
      caller feeds them, as `examples/ascii_simulation.rs` does
- [ ] Web-based visualization: an HTTP API exists in `analytics/web_api.rs`
      but has no call sites and no front end
- [x] Bevy front end (`cargo run --features bevy_gui --bin ebss_bevy`)
- [ ] Performance has not been profiled at scale

### Beyond the original plan
- [ ] Give world generation a seed, so runs are reproducible and three flaky
      tests become deterministic
- [ ] Feed the remaining percept channels: agents discover the world by sight
      now, but still cannot see each other or hear anything
- [ ] Feed a grown settlement: food regrows about four times slower than forty
      people eat it, so a quarter of settlements still starve out past twenty
      thousand ticks
- [ ] Characterise long-run behaviour past 100k ticks

## Core Concepts

### Behavior Trees
Agents maintain forests of behavior trees where successful patterns are reinforced over time. Each tree branch has a weight that increases with positive outcomes.

### Drive System
14 core drives motivate agent behavior, in the order they appear in
`DriveType`:
1. Hunger - Seek and consume food
2. Thirst - Find and drink water
3. Rest - Sleep and recover from fatigue
4. Shelter - Build or locate protective structures
5. Safety - Avoid threats, create defenses
6. Preparedness - Stockpile resources and tools
7. Industry - Mine, smelt, and process materials
8. Sustenance - Farm and produce food
9. Curiosity - Explore and learn
10. Social - Interact with other agents
11. Reproduction - Create offspring
12. Luxury - Seek rare or decorative items
13. Utility - Maintain tools and equipment
14. Construction - Build structures and infrastructure

### Memory
Agents remember:
- Spatial locations (resources, structures, landmarks)
- Storage contents with decay over time
- Social relationships and observed behaviors
- Discovered crafting recipes

### Learning
- **Trial & Error**: Random exploration with reinforcement
- **Observation**: Young agents copy experienced agents
- **Inheritance**: Offspring receive pruned parent behavior trees

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-tarpaulin  # For code coverage

# Run tests in watch mode
cargo watch -x test

# Check code coverage
cargo tarpaulin --out Html
```

## Documentation

- [Software Design Document](EBSS_Software_Design_Document.docx) - Original architecture and specifications
- [PROJECT_STATUS.txt](PROJECT_STATUS.txt) - Measured state of the project: what builds, what runs, what does not
- [SIMULATION_AUDIT.md](SIMULATION_AUDIT.md) - Which subsystems the simulation loop actually drives
- [ISSUES_FOUND.md](ISSUES_FOUND.md) - Current known defects, with reproduction steps
- [TESTING.md](TESTING.md) - How to run and write tests
- [SETUP.md](SETUP.md) - Development environment setup
- [docs/VISUALIZATION.md](docs/VISUALIZATION.md) - Rendering and display
- API reference: `cargo doc --open` (there is no checked-in API document)
- Examples: 21 runnable programs in [examples/](examples/), starting with
  `basic_survival.rs`

## Research Applications

EBSS is designed for:
- AI benchmarking and algorithm comparison
- Multi-agent reinforcement learning research
- Evolutionary algorithm studies
- Social science simulations
- Game AI development
- Emergent behavior analysis

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Citation

If you use EBSS in your research, please cite:

```bibtex
@software{ebss2024,
  title={Emergent Behavior Society Simulator},
  author={Your Name},
  year={2024},
  url={https://github.com/yourusername/ebss-project}
}
```

## Acknowledgments

- Inspired by Dwarf Fortress's emergent complexity
- Based on behavior tree and drive system concepts from game AI research
- Built with the Rust ecosystem

## Contact

- Issues: [GitHub Issues](https://github.com/yourusername/ebss-project/issues)
- Discussions: [GitHub Discussions](https://github.com/yourusername/ebss-project/discussions)
- Email: your.email@example.com

---

**Note**: This project is in active development. APIs and features are subject to change.
