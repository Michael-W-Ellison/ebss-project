# EBSS Testing & Troubleshooting

This document describes how to use the test executable for easy testing and troubleshooting of the EBSS simulation.

## Quick Start

Run the test simulation with default settings:

```bash
cargo run --bin test_simulation
```

This will:
- Spawn 10 agents
- Run for 1000 ticks
- Report status every 100 ticks

## Command Line Options

Customize the simulation with command line arguments:

```bash
cargo run --bin test_simulation -- --agents <N> --ticks <T> --report <R>
```

### Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `--agents <N>` | Number of agents to spawn | 10 |
| `--ticks <T>` | Number of ticks to run | 1000 |
| `--report <R>` | Report status every R ticks | 100 |

### Examples

Run a small test with 5 agents for 500 ticks:
```bash
cargo run --bin test_simulation -- --agents 5 --ticks 500
```

Run a longer simulation with frequent reports:
```bash
cargo run --bin test_simulation -- --agents 20 --ticks 5000 --report 250
```

Test death mechanics over a long period:
```bash
cargo run --bin test_simulation -- --agents 15 --ticks 15000 --report 1000
```

## Output Information

The test executable provides comprehensive output about the simulation state:

### Initial Status
- Configuration summary
- World initialization
- Agent spawning confirmation
- Initial population statistics

### Periodic Reports (every R ticks)
- **Population Status**: Current population size, life stage distribution, births, deaths
- **Death Watch**: Warning list of agents in critical condition
  - Low health (<30)
  - Starving (days without food)
  - Low energy (<20)
  - Old age (>90% of max_age)

### Final Statistics
- Simulation duration
- Final population metrics
- Death and birth rates
- Detailed breakdown of surviving agents:
  - Age progress
  - Health status
  - Energy levels
  - Hunger state

## Death Mechanics Being Tested

The executable specifically monitors and reports on the integrated death mechanics:

### 1. **Old Age Death**
- Agents have a randomized `max_age` between 9,000-11,000 ticks
- Death occurs when `age >= max_age`
- Reported as "Old Age: X%" in Death Watch

### 2. **Starvation Death**
- Progressive health loss when agents don't eat
- Timeline:
  - 0-1 day: Normal metabolism
  - 1-3 days: Increased energy depletion
  - 3-7 days: Health degradation (0.1/tick)
  - 7+ days: Rapid health loss (1.0/tick)
- Reported as "Starving (Xd)" in Death Watch

### 3. **Body Damage Death**
- Health depletion from injuries, bleeding, poison, burns
- Critical injuries to head or torso cause instant death
- Reported as "Low Health: X" in Death Watch

## Interpreting Results

### Population Survival
- If population dies out before the simulation ends, you'll see: `SIMULATION ENDED: Population extinct at tick X`
- This indicates death mechanics are working (agents are dying faster than they can reproduce)

### Life Stage Distribution
Monitor how agents progress through life stages:
- **Infant**: 0-500 ticks (cannot reproduce)
- **Child**: 501-1500 ticks (learning rate 1.5x)
- **Adolescent**: 1501-2500 ticks (can reproduce, fertility 0.7x)
- **Adult**: 2501-8000 ticks (prime reproduction)
- **Elderly**: 8000+ ticks (reduced fertility 0.3x)

### Death Rate Analysis
Compare births vs deaths:
- Sustainable population: births ≥ deaths
- Declining population: deaths > births
- Growing population: births > deaths significantly

## Troubleshooting

### Population Dies Too Quickly
If all agents die before simulation ends:
1. Check average death rate in final statistics
2. Review Death Watch logs to see primary causes
3. Consider:
   - Starvation is too harsh (food mechanics not implemented)
   - Reproduction rate is too low
   - Initial health/energy too low

### No Deaths Occur
If no agents die during a long simulation:
1. Verify `age_tick()` is being called (check logs)
2. Ensure `process_deaths()` is removing dead agents
3. Run longer simulation (agents need ~9000 ticks to die of old age)

### Agents Not Aging
If life stages never change:
1. Verify `tick_with_time()` is called in population tick
2. Check that `age` is incrementing in agent state
3. Review `age_tick()` integration in agent.rs:616-622

## Testing Specific Mechanics

### Test Aging Death
```bash
# Run long enough for agents to reach max_age
cargo run --bin test_simulation -- --agents 10 --ticks 12000 --report 1000
```
Expected: Agents should start dying around tick 9000-11000

### Test Starvation (when food mechanics implemented)
```bash
# Short simulation to observe hunger progression
cargo run --bin test_simulation -- --agents 5 --ticks 2000 --report 200
```
Expected: Death Watch should show increasing starvation warnings

### Test Reproduction & Aging Together
```bash
# Long simulation with multiple generations
cargo run --bin test_simulation -- --agents 20 --ticks 20000 --report 1000
```
Expected: See births, deaths, and transitions through life stages

## Logging

Enable debug logging for more detailed output:

```bash
RUST_LOG=debug cargo run --bin test_simulation -- --agents 5 --ticks 500
```

Log levels:
- `error`: Only errors
- `warn`: Warnings and errors
- `info`: General information (default)
- `debug`: Detailed debugging information
- `trace`: Very verbose output

## Build for Release

For faster execution, build in release mode:

```bash
cargo build --release --bin test_simulation
./target/release/test_simulation --agents 50 --ticks 50000 --report 5000
```

Release builds run significantly faster but take longer to compile.
