# Interval operations

This tutorial shows how to use `ladle.ops.intervals` for common genomic interval analyses.
All functions accept PyArrow, Polars, or Pandas DataFrames and return a `pyarrow.RecordBatch`.

## Input requirements

Every DataFrame must have columns named `chrom` (or `contig`/`chr`), `start`, and `end`.
Rename before calling if your schema differs:

```python
df = df.rename({"seqname": "chrom", "chromStart": "start", "chromEnd": "end"})
```

## Overlap between two interval sets

A typical workflow: find all BAM reads overlapping variant sites.

```python
import polars as pl
import ladle.ops.intervals as intervals
from ladle.io.bam import Reader as BamReader
from ladle.io.vcf import Reader as VcfReader

# Load BAM batch and create interval columns (0-based half-open)
with BamReader.from_path("sample.bam") as r:
    r.read_header()
    reads = (
        r.records_to_batch()
        .to_polars()
        .rename({"reference_sequence_id": "chrom"})
        .with_columns(end=pl.col("alignment_start") + pl.col("sequence").bin.count())
        .rename({"alignment_start": "start"})
    )

# Load VCF variants
with VcfReader.from_path("variants.vcf") as r:
    header = r.read_header()
    variants = (
        r.records_to_batch(header)
        .to_polars()
        .with_columns(
            start=pl.col("pos") - 1,
            end=pl.col("pos") - 1 + pl.col("ref").str.len_bytes(),
        )
        .rename({"chrom": "chrom"})
    )

result = pl.from_arrow(intervals.overlap(reads, variants))
print(f"{len(result)} read-variant overlaps found")
```

## Count how many variants fall in each read

```python
counts = pl.from_arrow(intervals.count_overlaps(reads, variants))
print(counts.filter(pl.col("count") > 0).head())
```

## Find the nearest gene for each variant

```python
genes = pl.read_csv("genes.bed", separator="\t",
                    new_columns=["chrom", "start", "end", "gene_name"])

nearest = pl.from_arrow(intervals.nearest(variants, genes))
# nearest contains a_* columns (variant) + b_* columns (nearest gene) + distance
print(nearest.select(["a_chrom", "a_start", "b_gene_name", "distance"]).head())
```

## Merge overlapping peaks

```python
peaks = pl.read_csv("peaks.bed", separator="\t",
                    new_columns=["chrom", "start", "end", "score"])

merged = pl.from_arrow(intervals.merge(peaks))
print(f"Reduced {len(peaks)} peaks to {len(merged)} merged regions")
```

## Compute coverage from alignments

```python
with BamReader.from_path("sample.bam") as r:
    r.read_header()
    reads = r.records_to_batch().to_polars()

# Build interval columns
reads = reads.with_columns(
    start=pl.col("alignment_start") - 1,
    end=pl.col("alignment_start") - 1 + pl.col("sequence").bin.count(),
    chrom=pl.col("reference_sequence_id").cast(pl.Utf8),
).filter(pl.col("start").is_not_null())

cov = pl.from_arrow(intervals.coverage(reads))
# chrom / start / end / depth
print(cov.filter(pl.col("depth") > 10).head())
```

## Subtract blacklisted regions

```python
blacklist = pl.read_csv("blacklist.bed", separator="\t",
                        new_columns=["chrom", "start", "end"])

clean_peaks = pl.from_arrow(intervals.subtract(peaks, blacklist))
print(f"{len(clean_peaks)} peaks after removing blacklist regions")
```

## Complement: find uncovered regions

```python
chrom_sizes = {"chr1": 248956422, "chr2": 242193529}

gaps = pl.from_arrow(intervals.complement(merged, chrom_sizes=chrom_sizes))
print(f"{len(gaps)} uncovered regions")
```

## Per-sample analysis with on_cols

Use `on_cols` to run any operation independently within groups:

```python
# peaks has a "sample" column
per_sample_merged = pl.from_arrow(
    intervals.merge(peaks, on_cols=["sample"])
)
```
