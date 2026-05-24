# ladle

Python bindings to the [noodles](https://github.com/zaeleus/noodles) bioinformatics I/O library, via PyO3. The API mirrors the noodles crate hierarchy.

## Install

```bash
pip install ladle

# Optional DataFrame extras
pip install "ladle[arrow]"    # pyarrow  — RecordBatch.to_arrow()
pip install "ladle[polars]"   # polars   — RecordBatch.to_polars()
pip install "ladle[pandas]"   # pandas   — RecordBatch.to_pandas()
```

> Not yet on PyPI — build from source (see [Development](#development)).

## Requirements

- Python 3.11+
- Rust 1.88+ (source builds only)

## Modules

```
ladle.io.bam    # BAM: Reader, IndexedReader, Writer, RecordBatch
ladle.io.vcf    # VCF: Reader, Writer, RecordBatch, Header, Record
ladle.io.bcf    # BCF: Reader, IndexedReader, Writer, RecordBatch
ladle.io.sam    # SAM: Reader, Writer, RecordBatch, Header, Flags
ladle.io.bgzf   # BGZF block-gzip: Reader, Writer
ladle.io.core   # Position, Region, Interval
ladle.ops.intervals  # overlap(a, b), nearest(query, target)
```

Full API reference: [windowgenerator.github.io/ladle](https://windowgenerator.github.io/ladle)

---

## Examples

### BAM — per-record iteration

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    header = r.read_header()
    for record in r:
        name  = record.name()              # bytes | None
        flags = record.flags()
        start = record.alignment_start()   # core.Position | None
        print(name, int(flags), int(start) if start else None)
```

### BAM — columnar batch

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    r.read_header()
    batch = r.records_to_batch()

df  = batch.to_polars()  # pip install ladle[polars]
df  = batch.to_arrow()   # pip install ladle[arrow]
pdf = batch.to_pandas()  # pip install ladle[pandas]
```

### BCF — columnar batch

```python
from ladle.io.bcf import Reader

with Reader.from_path("variants.bcf") as r:
    header = r.read_header()
    batch  = r.records_to_batch(header)

df = batch.to_polars()
print(df.head())
# shape: (N, 6+)
# ┌───────┬──────┬──────┬─────┬─────┬──────┬──────────┬──────────┐
# │ chrom ┆ pos  ┆ id   ┆ ref ┆ alt ┆ qual ┆ INFO_DP  ┆ INFO_AF  │
# │ str   ┆ i32  ┆ str  ┆ str ┆ str ┆ f32  ┆ i32      ┆ f32      │
```

### BCF — per-record iteration

```python
from ladle.io.bcf import Reader

with Reader.from_path("variants.bcf") as r:
    header = r.read_header()
    for record in r:
        chrom = record.reference_sequence_name(header)
        pos   = record.variant_start()    # core.Position | None
        alts  = record.alternate_bases()  # list[str]
        qual  = record.quality_score()    # float | None
        print(chrom, int(pos) if pos else None, alts, qual)
```

### BCF — indexed region query

```python
from ladle.io.bcf import IndexedReader
from ladle.io.core import Region

with IndexedReader.from_path("variants.bcf") as r:
    header = r.read_header()
    region = Region.parse("chr1:1000000-2000000")
    for record in r.query(header, region):
        print(record.reference_sequence_name(header), record.variant_start())
```

### VCF — read from Python file descriptor and overlap with BCF

A common workflow: load variants from a VCF (via a file descriptor) and
annotations from a BCF, then find all pairs of variants that overlap in
genomic coordinates.

```python
import polars as pl
import ladle.ops.intervals as intervals
from ladle.io.vcf import Reader as VcfReader
from ladle.io.bcf import Reader as BcfReader

# VCF pos is 1-based; intervals.overlap expects 0-based half-open [start, end)
def add_interval_cols(df):
    return df.with_columns(
        start=pl.col("pos") - 1,
        end=pl.col("pos") - 1 + pl.col("ref").str.len_bytes(),
    )

with VcfReader.from_fd(open("calls.vcf")) as r:
    h = r.read_header()
    vcf = add_interval_cols(r.records_to_batch(h).to_polars())

with BcfReader.from_path("annotations.bcf") as r:
    h = r.read_header()
    bcf = add_interval_cols(r.records_to_batch(h).to_polars())

overlapping = pl.from_arrow(intervals.overlap(vcf, bcf))
print(f"Found {len(overlapping)} overlapping variant pairs")
print(overlapping.head())
```

To carry only specific columns through, pass a narrower `select()` —
they will appear prefixed `a_` / `b_` in the result:

```python
overlapping = pl.from_arrow(intervals.overlap(
    vcf.select("chrom", "start", "end", "id", "INFO_AF"),
    bcf.select("chrom", "start", "end", "id"),
))
# columns: a_chrom, a_start, a_end, a_id, a_INFO_AF, b_chrom, b_start, b_end, b_id
```

---

## Development

```bash
# Create venv and install dev dependencies
python3.11 -m venv .venv
.venv/bin/pip install maturin pytest pyarrow polars pandas

# Build and install the Rust extension in develop mode
VIRTUAL_ENV="$(pwd)/.venv" .venv/bin/maturin develop

# Run tests
.venv/bin/pytest
```
