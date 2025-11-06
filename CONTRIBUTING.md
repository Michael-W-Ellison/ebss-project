# Contributing to EBSS

Thank you for your interest in contributing to the Emergent Behavior Society Simulator!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/ebss-project.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Commit: `git commit -m "Add feature: description"`
7. Push: `git push origin feature/your-feature-name`
8. Create a Pull Request

## Development Guidelines

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Run clippy: `cargo clippy`
- Add documentation for public APIs
- Write tests for new features

### Testing

- Write unit tests for individual functions
- Add integration tests for module interactions
- Ensure all tests pass before submitting PR
- Aim for >80% code coverage

### Commit Messages

- Use clear, descriptive commit messages
- Start with a verb (Add, Fix, Update, Remove)
- Reference issues when applicable: `Fix #123: description`

### Pull Requests

- Provide a clear description of changes
- Link to related issues
- Ensure CI passes
- Request review from maintainers

## Areas for Contribution

- Core AI algorithms (behavior trees, learning)
- Environment plugins (new world types)
- Performance optimization
- Documentation and examples
- Bug fixes
- Testing

## Questions?

Open an issue or start a discussion!
