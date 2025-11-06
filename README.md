# Emergent Behavior Society Simulator (EBSS)

A general-purpose AI platform for simulating societies of autonomous agents that learn and adapt through behavioral evolution.

## Overview

EBSS provides a modular framework where agents develop complex behaviors through:
- **Weighted Behavior Trees**: Learned decision-making patterns that evolve with experience
- **Drive-Based Motivation**: 13 core drives (hunger, safety, curiosity, social, etc.) creating dynamic priorities
- **Genetic Inheritance**: Offspring inherit successful behavioral patterns from parents
- **Memory Systems**: Agents remember locations, storage contents, and other agents
- **Observational Learning**: Young agents learn by following experienced agents
- **Modular Environments**: Plugin architecture for different world rules and game mechanics

Unlike game-specific implementations, EBSS is environment-agnostic, allowing researchers and developers to plug in different rule systems (Minecraft-style survival, Dwarf Fortress-inspired societies, medieval simulations, or entirely novel environments) while maintaining the same core AI architecture.

## Project Status

**Current Phase**: Foundation Development (Phase 1)

This project is in early development. See the [Software Design Document](docs/Software_Design_Document.docx) for complete specifications.

## Key Features

- ✅ Behavior Tree Learning: Agents build and evolve decision trees through experience
- ✅ Drive Architecture: 13 core drives create emergent behavior patterns
- 🚧 Genetic Inheritance: Offspring learn from successful parent strategies
- 🚧 Environment Abstraction: Plugin system for different world types
- 📋 Memory Systems: Knowledge persistence with decay
- 📋 Social Learning: Observation and imitation mechanics
- 📋 Analytics: Emergence detection and behavior analysis

Legend: ✅ Implemented | 🚧 In Progress | 📋 Planned

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

### Phase 1: Core Foundation (Months 1-4) ⏳
- [x] Project structure and build system
- [ ] Basic behavior tree implementation
- [ ] Core drive system (5 drives)
- [ ] Simple grid-based world
- [ ] Agent actions and learning
- [ ] ASCII visualization

### Phase 2: Environment Abstraction (Months 5-8)
- [ ] Plugin architecture
- [ ] Material property system
- [ ] Template-based crafting
- [ ] Minecraft-style environment
- [ ] Tool effectiveness calculations

### Phase 3: Social Systems (Months 9-12)
- [ ] Reproduction mechanics
- [ ] Genetic inheritance
- [ ] Observational learning
- [ ] Social memory
- [ ] All 13 drives implemented

### Phase 4: Analytics and Polish (Months 13-18)
- [ ] Data logging and analysis
- [ ] Web-based visualization
- [ ] Emergence detection
- [ ] Performance optimization
- [ ] Additional environment plugins

## Core Concepts

### Behavior Trees
Agents maintain forests of behavior trees where successful patterns are reinforced over time. Each tree branch has a weight that increases with positive outcomes.

### Drive System
13 core drives motivate agent behavior:
1. Hunger - Seek and consume food
2. Rest - Find shelter and sleep
3. Shelter - Build or locate protective structures
4. Safety - Avoid threats, create defenses
5. Preparedness - Stockpile resources and tools
6. Industry - Mine, smelt, and process materials
7. Sustenance - Farm and produce food
8. Curiosity - Explore and learn
9. Social - Interact with other agents
10. Reproduction - Create offspring
11. Luxury - Seek rare or decorative items
12. Utility - Maintain tools and equipment
13. Construction - Build structures and infrastructure

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

- [Software Design Document](docs/Software_Design_Document.docx) - Complete system architecture and specifications
- [API Documentation](docs/api/README.md) - Detailed API reference
- [Environment Plugin Guide](docs/environment_plugins.md) - Creating custom environments
- [Examples](examples/README.md) - Tutorials and example simulations

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
