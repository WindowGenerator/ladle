# Quick start

## Requirements

- Python 3.11+
- Rust 1.88+ (for building from source)

## Install from source

```bash
git clone https://github.com/WindowGenerator/ladle.git
cd ladle

# Create a virtual environment
python3.11 -m venv .venv
source .venv/bin/activate

# Install build tool and runtime dependencies
pip install maturin pyarrow polars pandas pytest

# Build both Rust extensions and install in develop mode
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path ladle-io/Cargo.toml
VIRTUAL_ENV="$(pwd)/.venv" maturin develop --manifest-path ladle-ops/Cargo.toml
```

Or with [just](https://github.com/casey/just):

```bash
just build
```

## Verify the install

```python
import ladle.io.bam as bam
import ladle.io.vcf as vcf
import ladle.ops.intervals as intervals

print(bam.Reader)         # <class 'ladle.io.bam.Reader'>
print(intervals.overlap)  # <built-in function overlap>
```

## Read a BAM file

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    header = r.read_header()
    for record in r:
        print(record.name(), record.alignment_start())
```

## Load BAM into a Polars DataFrame

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    r.read_header()
    batch = r.records_to_batch()

df = batch.to_polars()
print(df.head())
```

## Find overlapping intervals

```python
import polars as pl
import ladle.ops.intervals as intervals

a = pl.DataFrame({"chrom": ["chr1", "chr1"], "start": [0, 10], "end": [15, 25]})
b = pl.DataFrame({"chrom": ["chr1"], "start": [12], "end": [20]})

result = pl.from_arrow(intervals.overlap(a, b))
print(result)
```

## Next steps

- [Tutorials](tutorials/index.md) — worked examples for common workflows
- [API Reference](api/bam.md) — full class and function documentation
