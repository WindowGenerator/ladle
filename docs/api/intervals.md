# ladle.ops.intervals

Genomic interval operations backed by a Rust engine (Rayon parallel, zero-copy Arrow I/O).
All functions accept any of **PyArrow**, **Polars**, or **Pandas** DataFrames and return a
`pyarrow.RecordBatch`.

## Column name requirements

Every input DataFrame must contain columns named:

| Semantic | Accepted column names |
|---|---|
| Chromosome | `chrom`, `contig`, `chr` |
| Start (0-based) | `start`, `pos` |
| End (exclusive) | `end`, `stop` |

Rename before calling if your schema differs:

```python
# Polars
df = df.rename({"seqname": "chrom", "chromStart": "start", "chromEnd": "end"})

# Pandas
df = df.rename(columns={"seqname": "chrom", "chromStart": "start", "chromEnd": "end"})

# PyArrow
batch = batch.rename_columns({"seqname": "chrom", "chromStart": "start", "chromEnd": "end"})
```

---

## overlap

Find all pairs of intervals from `a` and `b` that overlap.

**Overlap condition:** `a.start < b.end` and `b.start < a.end` (half-open intervals).
Cross-chromosome pairs are never considered overlapping.

```python
import polars as pl
import ladle.ops.intervals as intervals

a = pl.DataFrame({"chrom": ["chr1", "chr1"], "start": [1, 5], "end": [5, 10]})
b = pl.DataFrame({"chrom": ["chr1"], "start": [3], "end": [7]})

result = pl.from_arrow(intervals.overlap(a, b))
# shape: (2, 6)
# ┌──────────┬──────────┬────────┬──────────┬──────────┬────────┐
# │ a_chrom  ┆ a_start  ┆ a_end  ┆ b_chrom  ┆ b_start  ┆ b_end  │
# │ str      ┆ i64      ┆ i64    ┆ str      ┆ i64      ┆ i64    │
# ╞══════════╪══════════╪════════╪══════════╪══════════╪════════╡
# │ chr1     ┆ 1        ┆ 5      ┆ chr1     ┆ 3        ┆ 7      │
# │ chr1     ┆ 5        ┆ 10     ┆ chr1     ┆ 3        ┆ 7      │
# └──────────┴──────────┴────────┴──────────┴──────────┴────────┘
```

Use `how="semi"` to filter `a` to rows that have at least one overlap in `b`,
without duplicating rows:

```python
result = pl.from_arrow(intervals.overlap(a, b, how="semi"))
# Returns rows from a only; schema unchanged
```

---

## nearest

For each row in `query`, find the closest row in `target` on the same chromosome.

**Distance** is 0 for overlapping intervals, otherwise the number of bases between them.
When no same-chromosome target exists, all `b_*` columns are `null` and `distance` is `null`.

```python
query  = pl.DataFrame({"chrom": ["chr1", "chr2"], "start": [10, 50], "end": [20, 60]})
target = pl.DataFrame({"chrom": ["chr1", "chr1"], "start": [1, 30], "end": [8, 40]})

result = pl.from_arrow(intervals.nearest(query, target))
# ┌──────────┬──────────┬────────┬──────────┬──────────┬────────┬──────────┐
# │ a_chrom  ┆ a_start  ┆ a_end  ┆ b_chrom  ┆ b_start  ┆ b_end  ┆ distance │
# │ str      ┆ i64      ┆ i64    ┆ str      ┆ i64      ┆ i64    ┆ i64      │
# ╞══════════╪══════════╪════════╪══════════╪══════════╪════════╪══════════╡
# │ chr1     ┆ 10       ┆ 20     ┆ chr1     ┆ 30       ┆ 40     ┆ 11       │
# │ chr2     ┆ 50       ┆ 60     ┆ null     ┆ null     ┆ null   ┆ null     │
# └──────────┴──────────┴────────┴──────────┴──────────┴────────┴──────────┘
```

Use `on_cols` to group by additional key columns (e.g. sample ID) before finding the nearest:

```python
result = pl.from_arrow(intervals.nearest(query, target, on_cols=["sample_id"]))
```

---

## count_overlaps

For each row in `a`, count how many rows in `b` overlap it.
Returns all columns from `a` plus a `count` column (`UInt32`). Rows with no overlaps get `count=0`.

```python
a = pl.DataFrame({"chrom": ["chr1", "chr1"], "start": [0, 5], "end": [10, 15]})
b = pl.DataFrame({"chrom": ["chr1", "chr1", "chr1"], "start": [1, 6, 12], "end": [4, 9, 20]})

result = pl.from_arrow(intervals.count_overlaps(a, b))
# ┌───────┬───────┬─────┬───────┐
# │ chrom ┆ start ┆ end ┆ count │
# │ str   ┆ i64   ┆ i64 ┆ u32   │
# ╞═══════╪═══════╪═════╪═══════╡
# │ chr1  ┆ 0     ┆ 10  ┆ 2     │
# │ chr1  ┆ 5     ┆ 15  ┆ 3     │
# └───────┴───────┴─────┴───────┘
```

---

## cluster

Assign a `cluster_id` to each interval. Overlapping or adjacent intervals on the same
chromosome share the same ID (`UInt32`, 0-based).
Returns all columns from `a` plus `cluster_id`.

```python
a = pl.DataFrame({
    "chrom": ["chr1", "chr1", "chr1", "chr2"],
    "start": [0, 5, 20, 0],
    "end":   [10, 15, 30, 5],
})

result = pl.from_arrow(intervals.cluster(a))
# ┌───────┬───────┬─────┬────────────┐
# │ chrom ┆ start ┆ end ┆ cluster_id │
# │ str   ┆ i64   ┆ i64 ┆ u32        │
# ╞═══════╪═══════╪═════╪════════════╡
# │ chr1  ┆ 0     ┆ 10  ┆ 0          │
# │ chr1  ┆ 5     ┆ 15  ┆ 0          │
# │ chr1  ┆ 20    ┆ 30  ┆ 1          │
# │ chr2  ┆ 0     ┆ 5   ┆ 2          │
# └───────┴───────┴─────┴────────────┘
```

---

## merge

Merge overlapping intervals into non-overlapping spans.
Returns only `chrom`/`start`/`end` columns (extra metadata columns are dropped).

```python
a = pl.DataFrame({
    "chrom": ["chr1", "chr1", "chr1"],
    "start": [0, 5, 20],
    "end":   [10, 15, 30],
    "score": [1, 2, 3],   # dropped in output
})

result = pl.from_arrow(intervals.merge(a))
# ┌───────┬───────┬─────┐
# │ chrom ┆ start ┆ end │
# ╞═══════╪═══════╪═════╡
# │ chr1  ┆ 0     ┆ 15  │
# │ chr1  ┆ 20    ┆ 30  │
# └───────┴───────┴─────┘
```

---

## subtract

Return rows of `a` that have **no overlap** with any row in `b`.
Entire rows are kept or dropped — no interval clipping. Schema is identical to `a`.

```python
a = pl.DataFrame({"chrom": ["chr1", "chr1", "chr2"], "start": [0, 5, 0], "end": [5, 10, 5]})
b = pl.DataFrame({"chrom": ["chr1"], "start": [4], "end": [8]})

result = pl.from_arrow(intervals.subtract(a, b))
# Returns only the row chr2:0-5 (the two chr1 rows overlap b)
```

---

## complement

Return the gaps between intervals on each chromosome.

If `chrom_sizes` is provided, also emits leading gaps `[0, first_start)` and
trailing gaps `[last_end, chrom_size)`.
Returns only `chrom`/`start`/`end` columns.

```python
a = pl.DataFrame({
    "chrom": ["chr1", "chr1"],
    "start": [10, 50],
    "end":   [30, 70],
})

# Without chrom_sizes — only inter-interval gaps
result = pl.from_arrow(intervals.complement(a))
# ┌───────┬───────┬─────┐
# │ chrom ┆ start ┆ end │
# ╞═══════╪═══════╪═════╡
# │ chr1  ┆ 30    ┆ 50  │
# └───────┴───────┴─────┘

# With chrom_sizes — leading and trailing gaps included
result = pl.from_arrow(intervals.complement(a, chrom_sizes={"chr1": 100}))
# ┌───────┬───────┬─────┐
# │ chrom ┆ start ┆ end │
# ╞═══════╪═══════╪═════╡
# │ chr1  ┆ 0     ┆ 10  │
# │ chr1  ┆ 30    ┆ 50  │
# │ chr1  ┆ 70    ┆ 100 │
# └───────┴───────┴─────┘
```

---

## coverage

Compute per-base coverage depth as contiguous blocks.
Returns `chrom`/`start`/`end`/`depth` (`UInt32`). Zero-depth regions are not emitted.

```python
a = pl.DataFrame({
    "chrom": ["chr1", "chr1"],
    "start": [0, 5],
    "end":   [10, 15],
})

result = pl.from_arrow(intervals.coverage(a))
# ┌───────┬───────┬─────┬───────┐
# │ chrom ┆ start ┆ end ┆ depth │
# │ str   ┆ i64   ┆ i64 ┆ u32   │
# ╞═══════╪═══════╪═════╪═══════╡
# │ chr1  ┆ 0     ┆ 5   ┆ 1     │
# │ chr1  ┆ 5     ┆ 10  ┆ 2     │
# │ chr1  ┆ 10    ┆ 15  ┆ 1     │
# └───────┴───────┴─────┴───────┘
```

---

## Geometry operations

### expand

Expand intervals by subtracting `amount` from `start` and adding to `end`.
Use `start_amount`/`end_amount` to control each side independently.
Start is clamped to 0. Returns same schema as `a`.

```python
a = pl.DataFrame({"chrom": ["chr1"], "start": [100], "end": [200]})

result = pl.from_arrow(intervals.expand(a, amount=50))
# start=50, end=250

result = pl.from_arrow(intervals.expand(a, start_amount=10, end_amount=50))
# start=90, end=250
```

### shift

Translate all intervals by `amount` bases.
Positive = rightward, negative = leftward. Start is clamped to 0; interval width is preserved.
Returns same schema as `a`.

```python
result = pl.from_arrow(intervals.shift(a, amount=1000))
```

### flank

Generate flanking regions adjacent to each interval.

- `start=True` (default): flank **before** — `[interval.start - width, interval.start)`
- `start=False`: flank **after** — `[interval.end, interval.end + width)`

```python
a = pl.DataFrame({"chrom": ["chr1"], "start": [100], "end": [200]})

upstream   = pl.from_arrow(intervals.flank(a, width=50))            # [50, 100)
downstream = pl.from_arrow(intervals.flank(a, width=50, start=False)) # [200, 250)
```

### set_width

Resize each interval to exactly `width` bases.

| `anchor` | Behaviour |
|---|---|
| `"start"` (default) | keep start, set `end = start + width` |
| `"end"` | keep end, set `start = end - width` |
| `"center"` | keep midpoint, expand equally both sides |

```python
result = pl.from_arrow(intervals.set_width(a, width=100, anchor="center"))
```

### tile

Split each interval into fixed-size tiles of `width` bases.
The last tile may be smaller. All non-interval columns are repeated for each tile.

```python
a = pl.DataFrame({"chrom": ["chr1"], "start": [0], "end": [25], "name": ["region1"]})

result = pl.from_arrow(intervals.tile(a, width=10))
# start: 0, 10, 20  |  end: 10, 20, 25  |  name: region1, region1, region1
```

---

## Set operations

### disjoin

Split overlapping intervals into non-overlapping disjoint pieces.
Every output interval spans a unique depth-homogeneous region.
Returns only `chrom`/`start`/`end`.

```python
result = pl.from_arrow(intervals.disjoin(a))
```

### intersect_ranges

Return regions covered by **both** `a` and `b`.
Clips overlapping pairs to their intersection and merges the result.
Returns only `chrom`/`start`/`end`.

```python
result = pl.from_arrow(intervals.intersect_ranges(a, b))
```

### union_ranges

Return regions covered by `a` **or** `b` (positional union).
Equivalent to merging the concatenation of both interval sets.
Returns only `chrom`/`start`/`end`.

```python
result = pl.from_arrow(intervals.union_ranges(a, b))
```

### setdiff_ranges

Return regions in `a` not covered by `b`.
Clips `a` intervals around all overlapping `b` intervals.
Returns only `chrom`/`start`/`end`.

```python
result = pl.from_arrow(intervals.setdiff_ranges(a, b))
```

---

## Sorting

### sort_bedframe

Sort intervals by `(chrom, start)`.
With `natural_chrom_order=True` (default), chromosomes are sorted numerically by suffix
(`chr1 < chr2 < chr10`), not lexicographically.

```python
result = pl.from_arrow(intervals.sort_bedframe(a))
result = pl.from_arrow(intervals.sort_bedframe(a, natural_chrom_order=False))
```

---

## on_cols — grouping key

Most two-input functions (`nearest`, `count_overlaps`, `cluster`, `merge`, `subtract`,
`complement`, `coverage`, `expand`, `shift`, `flank`, `set_width`, `tile`, `disjoin`)
accept an `on_cols` parameter.

When set, operations are performed independently within each group defined by those columns —
useful for per-sample or per-strand analysis:

```python
# Count overlaps separately per sample
result = pl.from_arrow(
    intervals.count_overlaps(a, b, on_cols=["sample_id"])
)
```

---

## API Reference

::: ladle.ops.intervals
