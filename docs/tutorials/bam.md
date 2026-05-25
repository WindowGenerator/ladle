# Reading alignment files

This tutorial covers reading BAM and SAM files with ladle — from simple record iteration
to columnar batch export and indexed region queries.

## Sequential record iteration (BAM)

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    header = r.read_header()
    for record in r:
        name  = record.name()             # bytes | None
        flags = record.flags()
        start = record.alignment_start()  # core.Position | None
        seq   = record.sequence()         # bytes

        print(name, int(flags), int(start) if start else None)
```

## Checking flags

[`Flags`][ladle.io.sam.Flags] is a bitset with convenience predicates:

```python
flags = record.flags()

if flags.is_unmapped():
    continue
if flags.is_reverse_complemented():
    print("reverse strand")
if flags.is_duplicate():
    print("PCR duplicate")

# Raw bitwise access
is_primary = not (flags.is_secondary() or flags.is_supplementary())
```

## Export all records to a DataFrame

```python
from ladle.io.bam import Reader

with Reader.from_path("sample.bam") as r:
    r.read_header()
    batch = r.records_to_batch()

# Pick your DataFrame library
df = batch.to_polars()
df = batch.to_arrow()
df = batch.to_pandas()
```

The batch contains these columns:

| Column | Type | Description |
|---|---|---|
| `name` | `LargeBinary` | Read name (QNAME) |
| `flags` | `UInt16` | SAM flags |
| `reference_sequence_id` | `Int32` | 0-based ref seq index (null if unmapped) |
| `alignment_start` | `Int32` | 1-based start (null if unmapped) |
| `mapping_quality` | `UInt8` | MAPQ (null if 255) |
| `cigar` | `LargeBinary` | Raw CIGAR bytes |
| `mate_reference_sequence_id` | `Int32` | Mate ref seq index |
| `mate_alignment_start` | `Int32` | Mate 1-based start |
| `template_length` | `Int32` | TLEN |
| `sequence` | `LargeBinary` | SEQ |
| `quality_scores` | `LargeBinary` | Raw Phred QUAL bytes |

## Iterate back from a batch

If you need to go back to record objects after loading the batch:

```python
with Reader.from_path("sample.bam") as r:
    r.read_header()
    batch = r.records_to_batch()

for record in batch.to_iterator():
    print(record.name())
```

## Region query with a BAI index

Use [`IndexedReader`][ladle.io.bam.IndexedReader] to fetch only reads that overlap a
genomic region. A `.bai` index file must exist next to the BAM file.

```python
from ladle.io.bam import IndexedReader
from ladle.io.core import Region, Interval

with IndexedReader.from_path("sample.bam") as r:
    header = r.read_header()
    region = Region(b"chr1", Interval(1_000_000, 2_000_000))
    for record in r.query(header, region):
        print(record.name(), record.alignment_start())
```

Or parse the region from a string:

```python
region = Region.parse("chr1:1000000-2000000")
```

## Reading SAM files

[`sam.Reader`][ladle.io.sam.Reader] has the same interface as `bam.Reader`:

```python
from ladle.io.sam import Reader

with Reader.from_path("sample.sam") as r:
    header = r.read_header()
    for record in r:
        print(record.name(), record.reference_sequence_name())
```

Note that SAM records use reference sequence **name** (bytes) rather than an integer ID.

## Writing BAM

```python
from ladle.io.bam import Reader, Writer

with Reader.from_path("input.bam") as r:
    header = r.read_header()
    with Writer.from_path("output.bam") as w:
        w.write_header(header)
        for record in r:
            if not record.flags().is_unmapped():
                w.write_record(header, record)
```
