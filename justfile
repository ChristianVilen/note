# Build debug binary
build:
    cargo build

# Build release binary
release:
    cargo build --release

# Run the app
run:
    cargo run

# Run all tests
test:
    cargo test

# Install to ~/.cargo/bin
install:
    cargo install --path .

# Check for compile errors without building
check:
    cargo check

# Format code
fmt:
    cargo fmt

# Run clippy lints
lint:
    cargo clippy

# Format + lint
ci: fmt lint test
