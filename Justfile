venv := ".venv"
maturin := venv + "/bin/maturin"
pytest := venv + "/bin/pytest"
ruff := venv + "/bin/ruff"

# List available recipes
default:
    @just --list

# Build both Rust extension modules (debug)
build:
    VIRTUAL_ENV={{venv}} {{maturin}} develop --manifest-path ladle-io/Cargo.toml
    VIRTUAL_ENV={{venv}} {{maturin}} develop --manifest-path ladle-ops/Cargo.toml

# Build both Rust extension modules (release)
build-release:
    VIRTUAL_ENV={{venv}} {{maturin}} develop --release --manifest-path ladle-io/Cargo.toml
    VIRTUAL_ENV={{venv}} {{maturin}} develop --release --manifest-path ladle-ops/Cargo.toml

# Run Python tests
test: build
    {{pytest}}
    cargo test --workspace

# Lint Python and auto-fix
lint-python-fix:
    {{ruff}} check --fix python/ tests/

# Run all linters
lint:
    {{ruff}} check python/ tests/
    VIRTUAL_ENV=`pwd`/{{venv}} cargo clippy --workspace -- -D warnings

# Format everything
fmt:
    {{ruff}} check --fix python/ tests/
    {{ruff}} format python/ tests/
    cargo fmt --all

# Build wheel for distribution
wheel:
    {{maturin}} build --release --manifest-path ladle-io/Cargo.toml
    {{maturin}} build --release --manifest-path ladle-ops/Cargo.toml

# Remove build artifacts
clean:
    cargo clean
    find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
    find . -type d -name "*.egg-info" -exec rm -rf {} + 2>/dev/null || true
