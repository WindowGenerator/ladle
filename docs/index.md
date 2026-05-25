# ladle

**ladle** is a Python library providing fast, ergonomic bindings to the
[noodles](https://github.com/zaeleus/noodles) bioinformatics I/O library, built with
[PyO3](https://pyo3.rs/). It also includes a parallel genomic interval engine backed by
[Rayon](https://github.com/rayon-rs/rayon) with zero-copy [Apache Arrow](https://arrow.apache.org/) output.

## What ladle gives you

- **Read and write BAM, SAM, VCF, BCF, FASTQ, BGZF** — sequential and random-access (indexed) readers
- **Columnar batch export** — every reader can produce an Arrow `RecordBatch` consumable by Polars, Pandas, or PyArrow directly
- **Genomic interval operations** — overlap, nearest, count, merge, cluster, coverage, and more, all parallelised with Rayon
- **Zero-copy data exchange** — Arrow C Data Interface throughout; no serialisation overhead

## Modules

| Module | Contents |
|---|---|
| `ladle.io.bam` | `Reader`, `IndexedReader`, `Writer`, `RecordBatch` |
| `ladle.io.sam` | `Reader`, `Writer`, `RecordBatch`, `Header`, `Flags` |
| `ladle.io.vcf` | `Reader`, `Writer`, `RecordBatch`, `Header`, `Record` |
| `ladle.io.bcf` | `Reader`, `IndexedReader`, `Writer`, `RecordBatch` |
| `ladle.io.fastq` | `Reader`, `Writer`, `Record` |
| `ladle.io.bgzf` | `Reader`, `Writer`, `VirtualPosition` |
| `ladle.io.core` | `Position`, `Interval`, `Region` |
| `ladle.ops.intervals` | `overlap`, `nearest`, `count_overlaps`, `merge`, `cluster`, … |

## Install

```bash
pip install ladle

# Optional DataFrame extras
pip install "ladle[arrow]"   # pyarrow
pip install "ladle[polars]"  # polars
pip install "ladle[pandas]"  # pandas
```

!!! note
    ladle is not yet on PyPI — see [Quick start](quickstart.md) for building from source.
