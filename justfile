# Default target: list all recipes
default:
    @just --list

# Format the code automatically
fmt:
    cargo fmt

# Check formatting exactly as CI does without changing files
fmt-check:
    cargo fmt --all -- --check

# Check if the code compiles without building an executable
check:
    cargo check

# Run the linter (clippy) to catch common mistakes
lint:
    cargo clippy -- -D warnings

# Run all unit and integration tests (verbose)
test:
    cargo test --verbose

# Build the project in debug mode (verbose)
build:
    cargo build --verbose

# Build the project in release mode (optimized)
release:
    cargo build --release

# Clean the build artifacts
clean:
    cargo clean

# Install lapctl to your local cargo bin directory
install: release
    cargo install --path .
