from typing import Any


def overlap(a: Any, b: Any) -> Any:
    """
    Find all overlapping pairs of rows between `a` and `b`.

    Inputs accept pyarrow.RecordBatch, polars.DataFrame, or pandas.DataFrame.
    Both inputs must have columns resolvable as chromosome, start, and end:
      - chromosome: "chrom", "contig", or "chr"
      - start:      "start" or "pos" (0-based, half-open)
      - end:        "end" or "stop"

    Returns a pyarrow.RecordBatch with all columns from `a` prefixed "a_"
    followed by all columns from `b` prefixed "b_". One row per overlapping pair.
    Rows with no overlap are excluded (inner join semantics).
    Cross-chromosome pairs are never considered overlapping.
    Overlap condition: a.start < b.end and b.start < a.end (half-open intervals).
    Uses all available CPU cores (Rayon parallel).
    """
    ...

def nearest(query: Any, target: Any) -> Any:
    """
    For each row in `query`, find the nearest row in `target` on the same chromosome.

    Inputs accept pyarrow.RecordBatch, polars.DataFrame, or pandas.DataFrame.
    Both inputs must have columns resolvable as chromosome, start, and end.

    Returns a pyarrow.RecordBatch with one row per query row:
      - all columns from `query` prefixed "a_"
      - nearest target columns prefixed "b_" (null if no same-chromosome target)
      - "distance" Int64 column: 0 if intervals overlap, otherwise the minimum
        of |query.start - target.end| and |target.start - query.end|.
        Null if no same-chromosome target exists.

    When multiple target rows are equidistant, the one with the smaller
    row index is returned. Uses all available CPU cores (Rayon parallel).
    """
    ...

def count_overlaps(a: Any, b: Any) -> Any:
    """
    For each row in `a`, count how many rows in `b` it overlaps.

    Returns a pyarrow.RecordBatch with all columns from `a` plus a
    "count" UInt32 column. One row per input `a` row; count=0 when no overlap.
    """
    ...

def cluster(a: Any) -> Any:
    """
    Assign a cluster_id to each interval. Overlapping or adjacent intervals
    on the same chromosome share the same cluster_id (UInt32, 0-based).

    Returns all columns from `a` plus "cluster_id" UInt32.
    """
    ...

def merge(a: Any) -> Any:
    """
    Merge overlapping intervals into non-overlapping spans.

    Returns a pyarrow.RecordBatch with chrom/start/end columns only.
    Extra metadata columns are dropped (same semantics as bedtools merge).
    """
    ...

def subtract(a: Any, b: Any) -> Any:
    """
    Return rows of `a` that have no overlap with any row in `b`.

    Row-mode: entire `a` rows are kept or dropped; no interval clipping.
    Returns the same schema as `a`.
    """
    ...

def complement(a: Any, chrom_sizes: dict[str, int] | None = None) -> Any:
    """
    Return the gaps between intervals on each chromosome.

    If `chrom_sizes` is provided, also emits leading gaps [0, first_start)
    and trailing gaps [last_end, chrom_size).
    Returns chrom/start/end columns only.
    """
    ...

def coverage(a: Any) -> Any:
    """
    Compute per-base coverage depth as contiguous blocks.

    Returns chrom/start/end/depth (UInt32) columns.
    Depth is the number of input intervals covering each position.
    Zero-depth regions are not emitted.
    """
    ...

def expand(a: Any, amount: int = 0, start_amount: int | None = None, end_amount: int | None = None) -> Any:
    """
    Expand intervals by subtracting from start and adding to end.

    `amount` sets both sides; `start_amount`/`end_amount` override individually.
    Start is clamped to 1. Returns same schema as `a`.
    """
    ...

def shift(a: Any, amount: int) -> Any:
    """
    Translate all intervals by `amount` bases (positive = right, negative = left).

    Start is clamped to 1; interval width is preserved when clamping occurs.
    Returns same schema as `a`.
    """
    ...

def sort_bedframe(a: Any, natural_chrom_order: bool = True) -> Any:
    """
    Sort intervals by (chrom, start).

    When `natural_chrom_order=True` (default), chromosomes are sorted
    numerically by suffix (chr1 < chr2 < chr10), not lexicographically.
    Returns same schema as `a`.
    """
    ...

def flank(a: Any, width: int, start: bool = True) -> Any:
    """
    Generate flanking regions adjacent to each interval.

    When `start=True` (default): flank before — [interval.start - width, interval.start).
    When `start=False`:          flank after  — [interval.end,            interval.end + width).
    Start clamped to 1. Returns same schema as `a` with replaced start/end.
    """
    ...

def set_width(a: Any, width: int, anchor: str = "start") -> Any:
    """
    Resize each interval to `width` bases.

    `anchor` controls which end is fixed:
      - "start"  (default): keep start, set end = start + width
      - "end":              keep end,   set start = end - width
      - "center":           keep midpoint, expand equally both sides
    Returns same schema as `a`.
    """
    ...

def tile(a: Any, width: int) -> Any:
    """
    Split each interval into fixed-size tiles of `width` bases.

    The last tile may be smaller. All non-interval columns are repeated
    for each tile. Returns same schema as `a`.
    """
    ...

def disjoin(a: Any) -> Any:
    """
    Split overlapping intervals into non-overlapping disjoint pieces.

    Every output interval spans a unique depth-homogeneous region.
    Returns chrom/start/end columns only.
    """
    ...
