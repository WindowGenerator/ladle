import ladle


def test_version():
    assert ladle.__version__ == "0.1.0"


def test_submodules():
    for name in ("bam", "bgzf", "core", "sam"):
        assert hasattr(ladle, name)
