# EBSS Project Setup Guide

This guide will help you get the Emergent Behavior Society Simulator project up and running.

## Prerequisites

### Required
- **Rust 1.70 or later**: Install from [rustup.rs](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### Optional
- **Git**: For version control
- **VS Code** with rust-analyzer extension (recommended IDE)
- **Lua 5.4+**: For environment plugin development

## Initial Setup

### 1. Clone or Extract the Project

If you received this as a zip/archive:
```bash
unzip ebss-project.zip
cd ebss-project
```

If cloning from GitHub:
```bash
git clone https://github.com/yourusername/ebss-project.git
cd ebss-project
```

### 2. Verify Rust Installation

```bash
rustc --version
cargo --version
```

You should see version 1.70 or later.

### 3. Build the Project

```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (slower compile, optimized runtime)
cargo build --release
```

Expected output:
```
   Compiling ebss v0.1.0 (/path/to/ebss-project)
    Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

### 4. Run Tests

```bash
cargo test
```

All tests should pass. If you see failures, please open an issue.

### 5. Run the Example

```bash
cargo run --example basic_survival
```

You should see:
```
=== EBSS Basic Survival Example ===

Creating world...
Spawning agents...
  Agent 1 spawned
  Agent 2 spawned
  ...
```

## Project Structure Overview

```
ebss-project/
├── src/
│   ├── core/               # Behavior trees, drives, learning
│   │   ├── behavior_tree.rs
│   │   ├── drives.rs
│   │   ├── learning.rs
│   │   └── memory.rs
│   ├── agents/             # Agent implementation
│   │   ├── agent.rs
│   │   └── population.rs
│   ├── environment/        # Environment abstraction
│   ├── world/              # World simulation
│   ├── analytics/          # Data analysis
│   └── lib.rs             # Library entry point
├── examples/              # Example simulations
│   └── basic_survival.rs
├── tests/                 # Integration tests
├── docs/                  # Documentation
└── Cargo.toml            # Project configuration
```

## Development Workflow

### Running in Watch Mode

Install cargo-watch for automatic recompilation:
```bash
cargo install cargo-watch
cargo watch -x test
```

### Formatting Code

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Generating Documentation

```bash
cargo doc --open
```

This will generate and open the API documentation in your browser.

## Common Issues

### Issue: "error: linker 'cc' not found"

**Solution (Ubuntu/Debian):**
```bash
sudo apt-get install build-essential
```

**Solution (macOS):**
```bash
xcode-select --install
```

### Issue: Slow compilation

**Solution:** Use the release build or enable faster linker:
```bash
# Add to ~/.cargo/config.toml
[build]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

### Issue: Out of memory during compilation

**Solution:** Reduce parallel jobs:
```bash
cargo build -j 2
```

## Next Steps

1. **Read the Documentation**: Check `docs/Software_Design_Document.docx`
2. **Explore Examples**: Look at `examples/basic_survival.rs`
3. **Run Tests**: `cargo test` to understand the test patterns
4. **Start Contributing**: See `CONTRIBUTING.md`

## Development Tools

### Recommended VS Code Extensions
- rust-analyzer
- CodeLLDB (for debugging)
- crates (dependency management)
- Better TOML

### Recommended Tools
```bash
# Code coverage
cargo install cargo-tarpaulin

# Benchmarking
cargo install cargo-criterion

# Security audit
cargo install cargo-audit
```

## Getting Help

- **Documentation**: Check the `docs/` directory
- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions for questions
- **Examples**: Look in `examples/` directory

## License

This project is licensed under the MIT License - see LICENSE file.

---

**Ready to contribute?** Read `CONTRIBUTING.md` and start coding!
