# Reading variant files

This tutorial covers reading VCF and BCF files — per-record iteration, header inspection,
columnar batch export, and indexed region queries.

## Sequential record iteration (VCF)

```python
from ladle.io.vcf import Reader

with Reader.from_path("variants.vcf") as r:
    header = r.read_header()
    for record in r:
        chrom = record.reference_sequence_name()  # str
        pos   = record.variant_start()            # core.Position | None
        ref   = record.reference_bases()          # str
        alts  = record.alternate_bases()          # list[str]
        qual  = record.quality_score()            # float | None
        print(chrom, int(pos) if pos else None, ref, alts, qual)
```

## Inspecting the header

```python
with Reader.from_path("variants.vcf") as r:
    header = r.read_header()

print(header.sample_names())   # ['NA12878', 'NA12879']
print(header.contigs())        # {'chr1': 248956422, 'chr2': 242193529, ...}
print(header.info_fields())    # [('DP', 'Integer'), ('AF', 'Float'), ...]
print(header.format_fields())  # [('GT', 'String'), ('GQ', 'Integer'), ...]
```

## Reading INFO fields

```python
with Reader.from_path("variants.vcf") as r:
    header = r.read_header()
    for record in r:
        info = record.info(header)
        dp = info.get("DP")   # int | None
        af = info.get("AF")   # float | None
        print(dp, af)
```

## Export all records to a DataFrame

```python
from ladle.io.vcf import Reader

with Reader.from_path("variants.vcf") as r:
    header = r.read_header()
    batch  = r.records_to_batch(header)   # pass header to decode INFO columns

df = batch.to_polars()
print(df.head())
```

## Sequential record iteration (BCF)

BCF records need the header to resolve string maps:

```python
from ladle.io.bcf import Reader

with Reader.from_path("variants.bcf") as r:
    header = r.read_header()
    for record in r:
        chrom = record.reference_sequence_name(header)  # requires header
        pos   = record.variant_start()
        ref   = record.reference_bases()                # bytes (BCF)
        alts  = record.alternate_bases()
        print(chrom, int(pos) if pos else None)
```

## Export BCF to a DataFrame

```python
from ladle.io.bcf import Reader

with Reader.from_path("variants.bcf") as r:
    header = r.read_header()
    batch  = r.records_to_batch(header)

df = batch.to_polars()
```

## Indexed region query (BCF)

[`bcf.IndexedReader`][ladle.io.bcf.IndexedReader] uses a `.csi` index for random access:

```python
from ladle.io.bcf import IndexedReader
from ladle.io.core import Region, Interval

with IndexedReader.from_path("variants.bcf") as r:
    header = r.read_header()
    region = Region.parse("chr1:1000000-2000000")
    for record in r.query(header, region):
        print(record.reference_sequence_name(header), record.variant_start())
```

## Writing VCF

```python
from ladle.io.vcf import Reader, Writer

with Reader.from_path("input.vcf") as r:
    header = r.read_header()
    with Writer.from_path("output.vcf") as w:
        w.write_header(header)
        for record in r:
            if record.quality_score() is not None:
                w.write_record(header, record)
```

## Cross-format: VCF → BCF

```python
from ladle.io.vcf import Reader
from ladle.io.bcf import Writer

with Reader.from_path("variants.vcf") as r:
    header = r.read_header()
    with Writer.from_path("variants.bcf") as w:
        w.write_header(header)
        for record in r:
            w.write_vcf_record(header, record)
```
