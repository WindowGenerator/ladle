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

    Inputs accept pyarrow.RecordBatch, polars.DataFrame, or pandas.DataFrame.
    Both inputs must have columns resolvable as chromosome, start, and end:
      - chromosome: "chrom", "contig", or "chr"
      - start:      "start" or "pos" (half-open)
      - end:        "end" or "stop"

    Returns a pyarrow.RecordBatch with all columns from `a` plus a
    "count" UInt32 column. One row per input `a` row; never filters rows.
    Rows with no overlapping b-intervals get count=0.
    Cross-chromosome pairs are never counted.
    Uses all available CPU cores (Rayon parallel).
    """
    ...
