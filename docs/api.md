# API reference

## `ladle.core`

```python
from ladle.core import Position, Interval, Region
```

| Class | Constructor | Notes |
|---|---|---|
| `Position` | `Position(n: int)` | 1-based; `ValueError` if 0 |
| `Interval` | `Interval(start=None, end=None)` | both bounds optional |
| `Region` | `Region(name: str \| bytes, interval: Interval)` | |

`Position` — `get()`, `checked_add(n)`, `parse(s)`, `MIN`, `MAX`, full ordering + hash.

`Interval` — `start()`, `end()`, `contains(pos)`, `intersects(other)`, `parse(s)`.

`Region` — `name() -> bytes`, `start()`, `end()`, `interval()`, `parse(s)`.

---

## `ladle.bgzf`

```python
from ladle.bgzf import Reader, Writer, VirtualPosition
import ladle.bgzf as bgzf
```

| Symbol | Notes |
|---|---|
| `COMPRESSION_NONE`, `COMPRESSION_FAST`, `COMPRESSION_BEST` | integer constants |
| `VirtualPosition(compressed, uncompressed)` | `from_u64(n)`, `MIN`, `MAX` |
| `Reader.from_path(path)` | `read(size)`, `read_all()`, `read_line()`, `virtual_position()`, `seek(vpos)` |
| `Writer.from_path(path, compression_level=6)` | `write(data)`, `flush()`, `finish()`, `virtual_position()` |
| `bgzf.gzi.Index.from_path(path)` | `query(pos) -> VirtualPosition` |

Both `Reader` and `Writer` are context managers.

---

## `ladle.sam`

```python
from ladle.sam import Flags, MappingQuality, Header, Record, Reader, Writer
```

**`Flags`** — `Flags(bits: int)`

Constants: `SEGMENTED`, `PROPERLY_SEGMENTED`, `UNMAPPED`, `MATE_UNMAPPED`,
`REVERSE_COMPLEMENTED`, `MATE_REVERSE_COMPLEMENTED`, `FIRST_SEGMENT`,
`LAST_SEGMENT`, `SECONDARY`, `QC_FAIL`, `DUPLICATE`, `SUPPLEMENTARY`.

Predicates: `is_unmapped()`, `is_secondary()`, `is_supplementary()`, … (one per constant).

Operators: `&`, `|`, `int()`, `==`, hash.

**`MappingQuality`** — `MappingQuality(n: int)` · `get()` · `MIN` (0) · `MAX` (254) · full ordering.

**`Header`** — `Header()` · `parse(s: str)` · `reference_sequences() -> dict[bytes, int]` · `str()`.

**`Record`** fields:

| Method | Return type |
|---|---|
| `name()` | `bytes \| None` |
| `flags()` | `Flags` |
| `reference_sequence_name()` | `bytes \| None` |
| `alignment_start()` | `Position \| None` |
| `mapping_quality()` | `MappingQuality \| None` |
| `cigar()` | `bytes` |
| `mate_reference_sequence_name()` | `bytes \| None` |
| `mate_alignment_start()` | `Position \| None` |
| `template_length()` | `int` |
| `sequence()` | `bytes` |
| `quality_scores()` | `bytes` |
| `data()` | `dict[bytes, int \| float \| bytes \| list]` |

**`Reader.from_path(path)`** — `read_header() -> Header`, then iterate records. Context manager.

**`Writer.from_path(path)`** — `write_header(header)`, `write_record(header, record)`. Context manager.

---

## `ladle.bam`

```python
from ladle.bam import Record, Reader, IndexedReader, Writer, RecordBatch
import ladle.bam as bam
```

**`Record`** — same fields as `sam.Record` except:
- `reference_sequence_id() -> int | None` (integer ID, not name)
- `mate_reference_sequence_id() -> int | None`

**`Reader.from_path(path)`** — `read_header() -> sam.Header`, iterate records, context manager.

```python
reader.records_to_batch() -> RecordBatch  # reads all remaining records into one batch
```

**`IndexedReader.from_path(path)`** — loads `.bai` index automatically.

```python
reader.query(header, region) -> Query     # region is ladle.core.Region
```

**`Writer.from_path(path)`** — `write_header(header)`, `write_record(header, record)`, `write_sam_record(header, sam_record)`.

**`bai.Index.read_from_path(path)`** — explicit BAI index handle.

**`RecordBatch`** — columnar representation of all records read by `records_to_batch()`.

| Method | Returns | Extra dependency |
|---|---|---|
| `to_arrow()` | `pyarrow.RecordBatch` | `pip install ladle[arrow]` |
| `to_polars()` | `polars.DataFrame` | `pip install ladle[polars]` |
| `to_pandas()` | `pandas.DataFrame` | `pip install ladle[pandas]` |
| `to_iterator()` | iterator of `bam.Record` | — |
| `len(batch)` | `int` | — |

BAM Arrow schema (11 columns):

| Column | Type | Nullable |
|---|---|---|
| `name` | `large_binary` | yes |
| `flags` | `uint16` | no |
| `reference_sequence_id` | `int32` | yes |
| `alignment_start` | `int32` | yes |
| `mapping_quality` | `uint8` | yes |
| `cigar` | `large_binary` | no |
| `mate_reference_sequence_id` | `int32` | yes |
| `mate_alignment_start` | `int32` | yes |
| `template_length` | `int32` | no |
| `sequence` | `large_binary` | no (4-bit packed) |
| `quality_scores` | `large_binary` | no |

---

## `ladle.vcf`

```python
from ladle.vcf import Header, Record, Reader, Writer, RecordBatch
```

**`Header`** — `Header()` · `parse(s: str)` · `sample_names() -> list[str]` · `contigs() -> dict[str, int | None]`.

**`Record`** fields:

| Method | Return type | Notes |
|---|---|---|
| `reference_sequence_name()` | `str` | |
| `variant_start()` | `Position \| None` | |
| `ids()` | `list[str]` | |
| `reference_bases()` | `str` | |
| `alternate_bases()` | `list[str]` | |
| `quality_score()` | `float \| None` | |
| `filters(header)` | `list[str]` | requires header |
| `info(header)` | `dict[str, ...]` | requires header |

**`Reader.from_path(path)`** — `read_header() -> Header`, iterate records, `records_to_batch() -> RecordBatch`. Context manager.

**`Writer.from_path(path)`** — `write_header(header)`, `write_record(header, record)`.

**`RecordBatch`** — same interface as `bam.RecordBatch` (`to_arrow`, `to_polars`, `to_pandas`, `to_iterator`, `len`).

VCF Arrow schema (6 columns):

| Column | Type | Nullable |
|---|---|---|
| `chrom` | `large_utf8` | no |
| `pos` | `int32` | yes |
| `id` | `large_utf8` | yes (`;`-joined, `None` if empty) |
| `ref` | `large_utf8` | no |
| `alt` | `large_utf8` | yes (`,`-joined, `None` if empty) |
| `qual` | `float32` | yes |

---

## `ladle.bcf`

```python
from ladle.bcf import Record, Reader, IndexedReader, Writer
from ladle.vcf import Header  # BCF shares VCF header
```

**`Record`** — same fields as `vcf.Record`. Methods that need format context (`reference_sequence_name`, `filters`, `info`) require `header` argument.

**`Reader.from_path(path)`** — `read_header() -> vcf.Header`, iterate records. Context manager.

**`IndexedReader.from_path(path)`** — loads `.csi` index automatically.

```python
reader.query(header, region) -> Query
```

**`Writer.from_path(path)`** — `write_header(header)`, `write_record(header, record)`, `write_vcf_record(header, vcf_record)`.

---

## `ladle.frames`

Convenience aliases — same objects, different import path.

```python
from ladle.frames import BamRecordBatch, VcfRecordBatch
# equivalent to:
# from ladle.bam import RecordBatch as BamRecordBatch
# from ladle.vcf import RecordBatch as VcfRecordBatch
```

<!-- extend: add new module section above this line -->
