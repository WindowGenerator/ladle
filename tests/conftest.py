import pytest


def pytest_configure(config):
    try:
        import ladle  # noqa: F401
    except ImportError:
        pytest.exit("ladle not built. Run: uv run maturin develop", returncode=3)
