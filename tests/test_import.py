import ladle


def test_version():
    assert ladle.__version__ == "0.1.0"


def test_submodules():
    import importlib

    for name in ("io", "ops"):
        assert hasattr(ladle, name)

    for dotted in (
        "ladle.io.bam",
        "ladle.io.bcf",
        "ladle.io.bgzf",
        "ladle.io.core",
        "ladle.io.fastq",
        "ladle.io.sam",
        "ladle.io.vcf",
        "ladle.ops.intervals",
    ):
        assert importlib.import_module(dotted) is not None
