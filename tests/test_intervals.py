import pytest

pa = pytest.importorskip("pyarrow")
import ladle.ops.intervals as intervals


def make_batch(rows, **extra):
    """rows: list of (chrom, start, end); extra: additional columns."""
    chroms = [r[0] for r in rows]
    starts = [r[1] for r in rows]
    ends   = [r[2] for r in rows]
    data = {"chrom": pa.array(chroms), "start": pa.array(starts, type=pa.int32()), "end": pa.array(ends, type=pa.int32())}
    for k, v in extra.items():
        data[k] = pa.array(v)
    return pa.record_batch(data)


# ---------------------------------------------------------------------------
# overlap
# ---------------------------------------------------------------------------

class TestOverlap:
    def test_basic_overlap(self):
        a = make_batch([("chr1", 0, 30)])
        b = make_batch([("chr1", 20, 60)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 1

    def test_no_overlap_gap(self):
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 20, 30)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0

    def test_half_open_boundary(self):
        # [0,10) and [10,20) should NOT overlap in half-open semantics
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 10, 20)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0

    def test_containment(self):
        a = make_batch([("chr1", 0, 100)])
        b = make_batch([("chr1", 10, 20)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 1

    def test_one_a_overlaps_three_b(self):
        a = make_batch([("chr1", 0, 100)])
        b = make_batch([("chr1", 10, 20), ("chr1", 30, 40), ("chr1", 50, 60)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 3

    def test_cross_chrom_no_overlap(self):
        a = make_batch([("chr1", 0, 100)])
        b = make_batch([("chr2", 0, 100)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0

    def test_mixed_chrom_only_matching(self):
        a = make_batch([("chr1", 0, 50), ("chr2", 0, 50)])
        b = make_batch([("chr1", 10, 60)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 1
        assert r["a_chrom"][0].as_py() == "chr1"

    def test_output_schema_prefixes(self):
        a = make_batch([("chr1", 0, 30)], score=pa.array([1.0]))
        b = make_batch([("chr1", 20, 60)], name=pa.array(["gene1"]))
        r = intervals.overlap(a, b)
        names = r.schema.names
        assert "a_chrom" in names and "a_start" in names and "a_end" in names
        assert "b_chrom" in names and "b_start" in names and "b_end" in names
        assert "a_score" in names
        assert "b_name" in names

    def test_column_types_preserved(self):
        a = make_batch([("chr1", 0, 30)])
        b = make_batch([("chr1", 20, 60)])
        r = intervals.overlap(a, b)
        assert r.schema.field("a_start").type == pa.int32()
        assert r.schema.field("b_start").type == pa.int32()

    def test_column_values_correct(self):
        a = make_batch([("chr1", 5, 25)], val=pa.array([42]))
        b = make_batch([("chr1", 10, 30)], val=pa.array([99]))
        r = intervals.overlap(a, b)
        assert r.num_rows == 1
        assert r["a_val"][0].as_py() == 42
        assert r["b_val"][0].as_py() == 99

    def test_empty_a(self):
        a = make_batch([])
        b = make_batch([("chr1", 0, 50)])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0
        assert "a_chrom" in r.schema.names
        assert "b_chrom" in r.schema.names

    def test_empty_b(self):
        a = make_batch([("chr1", 0, 50)])
        b = make_batch([])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0

    def test_both_empty(self):
        a = make_batch([])
        b = make_batch([])
        r = intervals.overlap(a, b)
        assert r.num_rows == 0

    def test_polars_input(self):
        pl = pytest.importorskip("polars")
        a = pl.DataFrame({"chrom": ["chr1"], "start": [0], "end": [30]})
        b = pl.DataFrame({"chrom": ["chr1"], "start": [20], "end": [60]})
        r = intervals.overlap(a, b)
        assert r.num_rows == 1

    def test_pandas_input(self):
        pd = pytest.importorskip("pandas")
        a = pd.DataFrame({"chrom": ["chr1"], "start": [0], "end": [30]})
        b = pd.DataFrame({"chrom": ["chr1"], "start": [20], "end": [60]})
        r = intervals.overlap(a, b)
        assert r.num_rows == 1

    def test_int64_start_end(self):
        a = pa.record_batch({"chrom": ["chr1"], "start": pa.array([0], type=pa.int64()), "end": pa.array([30], type=pa.int64())})
        b = pa.record_batch({"chrom": ["chr1"], "start": pa.array([20], type=pa.int64()), "end": pa.array([60], type=pa.int64())})
        r = intervals.overlap(a, b)
        assert r.num_rows == 1

    def test_unknown_column_raises(self):
        a = pa.record_batch({"chromosome": ["chr1"], "begin": [0], "finish": [30]})
        b = make_batch([("chr1", 20, 60)])
        with pytest.raises(Exception):
            intervals.overlap(a, b)

    def test_large_inputs(self):
        import random
        rng = random.Random(42)
        n = 10_000
        rows_a = [("chr1", i * 10, i * 10 + 15) for i in range(n)]
        rows_b = [("chr1", rng.randint(0, n * 10), rng.randint(0, n * 10)) for _ in range(n)]
        rows_b = [(c, min(s, e), max(s, e) + 1) for c, s, e in rows_b]
        a = make_batch(rows_a)
        b = make_batch(rows_b)
        r = intervals.overlap(a, b)
        assert r.num_rows >= 0  # just ensure no crash


# ---------------------------------------------------------------------------
# nearest
# ---------------------------------------------------------------------------

class TestNearest:
    def test_overlap_distance_zero(self):
        q = make_batch([("chr1", 10, 30)])
        t = make_batch([("chr1", 20, 40)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["distance"][0].as_py() == 0

    def test_upstream(self):
        # query at [50,60), target at [0,10) — distance = 50 - 10 = 40
        q = make_batch([("chr1", 50, 60)])
        t = make_batch([("chr1", 0, 10)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["distance"][0].as_py() == 40

    def test_downstream(self):
        # query at [0,10), target at [50,60) — distance = 50 - 10 = 40
        q = make_batch([("chr1", 0, 10)])
        t = make_batch([("chr1", 50, 60)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["distance"][0].as_py() == 40

    def test_no_same_chrom_target(self):
        q = make_batch([("chr1", 0, 10)])
        t = make_batch([("chr2", 0, 10)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["distance"][0].as_py() is None
        assert r["b_chrom"][0].as_py() is None

    def test_picks_nearest(self):
        # query [50,60), targets: [0,10) dist=40, [80,90) dist=20
        q = make_batch([("chr1", 50, 60)])
        t = make_batch([("chr1", 0, 10), ("chr1", 80, 90)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["distance"][0].as_py() == 20
        assert r["b_start"][0].as_py() == 80

    def test_equidistant_smaller_start(self):
        # query [50,60), targets [0,40) dist=10 and [70,110) dist=10 → pick [0,40) (smaller start)
        q = make_batch([("chr1", 50, 60)])
        t = make_batch([("chr1", 0, 40), ("chr1", 70, 110)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 1
        assert r["b_start"][0].as_py() == 0

    def test_one_row_per_query(self):
        q = make_batch([("chr1", 0, 10), ("chr1", 50, 60), ("chr2", 0, 10)])
        t = make_batch([("chr1", 20, 30)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 3

    def test_output_schema(self):
        q = make_batch([("chr1", 0, 10)])
        t = make_batch([("chr1", 20, 30)])
        r = intervals.nearest(q, t)
        names = r.schema.names
        assert "a_chrom" in names
        assert "b_chrom" in names
        assert "distance" in names
        assert r.schema.field("distance").type == pa.int64()

    def test_distance_null_not_zero_for_unmatched(self):
        q = make_batch([("chr1", 0, 10)])
        t = make_batch([("chr2", 0, 10)])
        r = intervals.nearest(q, t)
        assert r["distance"][0].as_py() is None

    def test_chrom_isolation(self):
        q = make_batch([("chr1", 0, 10)])
        t = make_batch([("chr2", 0, 5)])  # wrong chrom
        r = intervals.nearest(q, t)
        assert r["b_chrom"][0].as_py() is None

    def test_empty_query(self):
        q = make_batch([])
        t = make_batch([("chr1", 0, 10)])
        r = intervals.nearest(q, t)
        assert r.num_rows == 0

    def test_empty_target(self):
        q = make_batch([("chr1", 0, 10), ("chr1", 20, 30)])
        t = make_batch([])
        r = intervals.nearest(q, t)
        assert r.num_rows == 2
        assert all(r["b_chrom"][i].as_py() is None for i in range(2))
        assert all(r["distance"][i].as_py() is None for i in range(2))


# ---------------------------------------------------------------------------
# count_overlaps
# ---------------------------------------------------------------------------

class TestCountOverlaps:
    def test_basic_counts(self):
        # a row overlaps 2 b-rows, then 1, then 0
        a = make_batch([("chr1", 0, 100), ("chr1", 0, 25), ("chr1", 50, 60)])
        b = make_batch([("chr1", 10, 20), ("chr1", 30, 40), ("chr1", 80, 90)])
        r = intervals.count_overlaps(a, b)
        assert r.num_rows == 3
        counts = [r["count"][i].as_py() for i in range(3)]
        assert counts[0] == 3  # [0,100) overlaps all three
        assert counts[1] == 1  # [0,25) overlaps [10,20)
        assert counts[2] == 0  # [50,60) overlaps nothing

    def test_zero_overlap(self):
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 20, 30)])
        r = intervals.count_overlaps(a, b)
        assert r["count"][0].as_py() == 0

    def test_cross_chrom_not_counted(self):
        a = make_batch([("chr1", 0, 100)])
        b = make_batch([("chr2", 0, 100)])
        r = intervals.count_overlaps(a, b)
        assert r["count"][0].as_py() == 0

    def test_output_preserves_a_columns(self):
        a = make_batch([("chr1", 0, 100)], score=pa.array([7]))
        b = make_batch([("chr1", 10, 20)])
        r = intervals.count_overlaps(a, b)
        assert "chrom" in r.schema.names
        assert "start" in r.schema.names
        assert "end" in r.schema.names
        assert "score" in r.schema.names
        assert "count" in r.schema.names

    def test_count_column_type(self):
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 0, 10)])
        r = intervals.count_overlaps(a, b)
        assert r.schema.field("count").type == pa.uint32()

    def test_one_row_per_a(self):
        a = make_batch([("chr1", 0, 10), ("chr1", 20, 30)])
        b = make_batch([("chr1", 5, 15)])
        r = intervals.count_overlaps(a, b)
        assert r.num_rows == 2

    def test_empty_a(self):
        a = make_batch([])
        b = make_batch([("chr1", 0, 50)])
        r = intervals.count_overlaps(a, b)
        assert r.num_rows == 0

    def test_empty_b(self):
        a = make_batch([("chr1", 0, 50), ("chr1", 10, 20)])
        b = make_batch([])
        r = intervals.count_overlaps(a, b)
        assert r.num_rows == 2
        assert all(r["count"][i].as_py() == 0 for i in range(2))

    def test_polars_input(self):
        pl = pytest.importorskip("polars")
        a = pl.DataFrame({"chrom": ["chr1"], "start": [0], "end": [100]})
        b = pl.DataFrame({"chrom": ["chr1"], "start": [10], "end": [20]})
        r = intervals.count_overlaps(a, b)
        assert r["count"][0].as_py() == 1

    def test_pandas_input(self):
        pd = pytest.importorskip("pandas")
        a = pd.DataFrame({"chrom": ["chr1"], "start": [0], "end": [100]})
        b = pd.DataFrame({"chrom": ["chr1"], "start": [10], "end": [20]})
        r = intervals.count_overlaps(a, b)
        assert r["count"][0].as_py() == 1

    def test_half_open_boundary(self):
        # [0,10) and [10,20) should NOT overlap
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 10, 20)])
        r = intervals.count_overlaps(a, b)
        assert r["count"][0].as_py() == 0

    def test_multiple_chroms(self):
        a = make_batch([("chr1", 0, 50), ("chr2", 0, 50)])
        b = make_batch([("chr1", 10, 20), ("chr2", 10, 20), ("chr2", 30, 40)])
        r = intervals.count_overlaps(a, b)
        counts = {r["chrom"][i].as_py(): r["count"][i].as_py() for i in range(2)}
        assert counts["chr1"] == 1
        assert counts["chr2"] == 2


# ---------------------------------------------------------------------------
# cluster
# ---------------------------------------------------------------------------

class TestCluster:
    def test_basic(self):
        a = make_batch([("chr1", 1, 5), ("chr1", 3, 8), ("chr1", 10, 15)])
        r = intervals.cluster(a)
        ids = [r["cluster_id"][i].as_py() for i in range(3)]
        assert ids[0] == ids[1]        # first two overlap → same cluster
        assert ids[2] != ids[0]        # third is separate

    def test_adjacent_not_merged(self):
        # [0,10) and [10,20) touch but don't overlap → separate clusters
        a = make_batch([("chr1", 0, 10), ("chr1", 10, 20)])
        r = intervals.cluster(a)
        assert r["cluster_id"][0].as_py() != r["cluster_id"][1].as_py()

    def test_cross_chrom_separate(self):
        a = make_batch([("chr1", 0, 100), ("chr2", 0, 100)])
        r = intervals.cluster(a)
        assert r["cluster_id"][0].as_py() != r["cluster_id"][1].as_py()

    def test_preserves_columns(self):
        a = make_batch([("chr1", 0, 10)], score=pa.array([5]))
        r = intervals.cluster(a)
        assert "score" in r.schema.names
        assert "cluster_id" in r.schema.names

    def test_same_row_count(self):
        a = make_batch([("chr1", 0, 10), ("chr1", 5, 15), ("chr1", 20, 30)])
        r = intervals.cluster(a)
        assert r.num_rows == 3

    def test_cluster_id_type(self):
        a = make_batch([("chr1", 0, 10)])
        r = intervals.cluster(a)
        assert r.schema.field("cluster_id").type == pa.uint32()


# ---------------------------------------------------------------------------
# merge
# ---------------------------------------------------------------------------

class TestMerge:
    def test_basic(self):
        a = make_batch([("chr1", 1, 5), ("chr1", 3, 8), ("chr1", 10, 15)])
        r = intervals.merge(a)
        assert r.num_rows == 2
        assert r["start"][0].as_py() == 1
        assert r["end"][0].as_py() == 8
        assert r["start"][1].as_py() == 10
        assert r["end"][1].as_py() == 15

    def test_no_overlap(self):
        a = make_batch([("chr1", 0, 5), ("chr1", 10, 15)])
        r = intervals.merge(a)
        assert r.num_rows == 2

    def test_all_overlap(self):
        a = make_batch([("chr1", 0, 100), ("chr1", 10, 50), ("chr1", 80, 120)])
        r = intervals.merge(a)
        assert r.num_rows == 1
        assert r["start"][0].as_py() == 0
        assert r["end"][0].as_py() == 120

    def test_cross_chrom(self):
        a = make_batch([("chr1", 0, 50), ("chr1", 30, 80), ("chr2", 0, 50)])
        r = intervals.merge(a)
        assert r.num_rows == 2

    def test_output_schema(self):
        a = make_batch([("chr1", 0, 10)])
        r = intervals.merge(a)
        assert set(r.schema.names) == {"chrom", "start", "end"}

    def test_empty(self):
        a = make_batch([])
        r = intervals.merge(a)
        assert r.num_rows == 0


# ---------------------------------------------------------------------------
# subtract
# ---------------------------------------------------------------------------

class TestSubtract:
    def test_no_overlap_kept(self):
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr1", 20, 30)])
        r = intervals.subtract(a, b)
        assert r.num_rows == 1

    def test_overlap_removed(self):
        a = make_batch([("chr1", 0, 20)])
        b = make_batch([("chr1", 5, 15)])
        r = intervals.subtract(a, b)
        assert r.num_rows == 0

    def test_cross_chrom_kept(self):
        a = make_batch([("chr1", 0, 10)])
        b = make_batch([("chr2", 0, 10)])
        r = intervals.subtract(a, b)
        assert r.num_rows == 1

    def test_preserves_schema(self):
        a = make_batch([("chr1", 0, 10), ("chr1", 50, 60)], score=pa.array([1, 2]))
        b = make_batch([("chr1", 5, 15)])
        r = intervals.subtract(a, b)
        assert r.num_rows == 1
        assert r["score"][0].as_py() == 2

    def test_empty_b(self):
        a = make_batch([("chr1", 0, 10), ("chr1", 20, 30)])
        b = make_batch([])
        r = intervals.subtract(a, b)
        assert r.num_rows == 2

    def test_empty_a(self):
        a = make_batch([])
        b = make_batch([("chr1", 0, 10)])
        r = intervals.subtract(a, b)
        assert r.num_rows == 0


# ---------------------------------------------------------------------------
# complement
# ---------------------------------------------------------------------------

class TestComplement:
    def test_gap_between(self):
        a = make_batch([("chr1", 10, 30), ("chr1", 50, 70)])
        r = intervals.complement(a)
        # gap between: [30, 50)
        rows = [(r["chrom"][i].as_py(), r["start"][i].as_py(), r["end"][i].as_py()) for i in range(r.num_rows)]
        assert ("chr1", 30, 50) in rows

    def test_leading_gap_with_chrom_sizes(self):
        a = make_batch([("chr1", 10, 30)])
        r = intervals.complement(a, chrom_sizes={"chr1": 100})
        rows = [(r["start"][i].as_py(), r["end"][i].as_py()) for i in range(r.num_rows)]
        assert (0, 10) in rows
        assert (30, 100) in rows

    def test_no_gaps(self):
        a = make_batch([("chr1", 0, 50), ("chr1", 50, 100)])
        r = intervals.complement(a)
        assert r.num_rows == 0

    def test_output_schema(self):
        a = make_batch([("chr1", 10, 20), ("chr1", 30, 40)])
        r = intervals.complement(a)
        assert set(r.schema.names) == {"chrom", "start", "end"}

    def test_empty_input(self):
        a = make_batch([])
        r = intervals.complement(a)
        assert r.num_rows == 0


# ---------------------------------------------------------------------------
# coverage
# ---------------------------------------------------------------------------

class TestCoverage:
    def test_basic(self):
        a = make_batch([("chr1", 1, 5), ("chr1", 3, 8)])
        r = intervals.coverage(a)
        depths = {(r["start"][i].as_py(), r["end"][i].as_py()): r["depth"][i].as_py() for i in range(r.num_rows)}
        assert depths.get((1, 3)) == 1
        assert depths.get((3, 5)) == 2
        assert depths.get((5, 8)) == 1

    def test_no_overlap(self):
        a = make_batch([("chr1", 0, 5), ("chr1", 10, 15)])
        r = intervals.coverage(a)
        assert all(r["depth"][i].as_py() == 1 for i in range(r.num_rows))

    def test_output_schema(self):
        a = make_batch([("chr1", 0, 10)])
        r = intervals.coverage(a)
        assert set(r.schema.names) == {"chrom", "start", "end", "depth"}
        assert r.schema.field("depth").type == pa.uint32()

    def test_empty(self):
        a = make_batch([])
        r = intervals.coverage(a)
        assert r.num_rows == 0


# ---------------------------------------------------------------------------
# expand
# ---------------------------------------------------------------------------

class TestExpand:
    def test_symmetric(self):
        a = make_batch([("chr1", 10, 20)])
        r = intervals.expand(a, amount=5)
        assert r["start"][0].as_py() == 5
        assert r["end"][0].as_py() == 25

    def test_clamp_start(self):
        a = make_batch([("chr1", 3, 10)])
        r = intervals.expand(a, amount=5)
        assert r["start"][0].as_py() == 1

    def test_asymmetric(self):
        a = make_batch([("chr1", 10, 20)])
        r = intervals.expand(a, start_amount=2, end_amount=8)
        assert r["start"][0].as_py() == 8
        assert r["end"][0].as_py() == 28

    def test_preserves_schema(self):
        a = make_batch([("chr1", 10, 20)], score=pa.array([7]))
        r = intervals.expand(a, amount=1)
        assert "score" in r.schema.names
        assert r["score"][0].as_py() == 7


# ---------------------------------------------------------------------------
# shift
# ---------------------------------------------------------------------------

class TestShift:
    def test_shift_right(self):
        a = make_batch([("chr1", 10, 20)])
        r = intervals.shift(a, amount=5)
        assert r["start"][0].as_py() == 15
        assert r["end"][0].as_py() == 25

    def test_shift_left(self):
        a = make_batch([("chr1", 10, 20)])
        r = intervals.shift(a, amount=-5)
        assert r["start"][0].as_py() == 5
        assert r["end"][0].as_py() == 15

    def test_clamp_preserves_width(self):
        a = make_batch([("chr1", 3, 13)])
        r = intervals.shift(a, amount=-10)
        width = r["end"][0].as_py() - r["start"][0].as_py()
        assert r["start"][0].as_py() == 1
        assert width == 10


# ---------------------------------------------------------------------------
# sort_bedframe
# ---------------------------------------------------------------------------

class TestSortBedframe:
    def test_natural_order(self):
        a = make_batch([("chr10", 1, 2), ("chr2", 1, 2), ("chr1", 5, 6)])
        r = intervals.sort_bedframe(a)
        chroms = [r["chrom"][i].as_py() for i in range(3)]
        assert chroms == ["chr1", "chr2", "chr10"]

    def test_lexicographic_order(self):
        a = make_batch([("chr10", 1, 2), ("chr2", 1, 2), ("chr1", 5, 6)])
        r = intervals.sort_bedframe(a, natural_chrom_order=False)
        chroms = [r["chrom"][i].as_py() for i in range(3)]
        assert chroms == ["chr1", "chr10", "chr2"]

    def test_sorted_by_start_within_chrom(self):
        a = make_batch([("chr1", 30, 40), ("chr1", 10, 20)])
        r = intervals.sort_bedframe(a)
        assert r["start"][0].as_py() == 10
        assert r["start"][1].as_py() == 30

    def test_preserves_rows(self):
        a = make_batch([("chr1", 0, 10), ("chr2", 0, 10)])
        r = intervals.sort_bedframe(a)
        assert r.num_rows == 2


# ---------------------------------------------------------------------------
# flank
# ---------------------------------------------------------------------------

class TestFlank:
    def test_flank_left(self):
        a = make_batch([("chr1", 100, 200)])
        r = intervals.flank(a, width=50, start=True)
        assert r["start"][0].as_py() == 50
        assert r["end"][0].as_py() == 100

    def test_flank_right(self):
        a = make_batch([("chr1", 100, 200)])
        r = intervals.flank(a, width=50, start=False)
        assert r["start"][0].as_py() == 200
        assert r["end"][0].as_py() == 250

    def test_flank_clamp(self):
        a = make_batch([("chr1", 5, 20)])
        r = intervals.flank(a, width=100, start=True)
        assert r["start"][0].as_py() == 1

    def test_preserves_schema(self):
        a = make_batch([("chr1", 100, 200)], score=pa.array([3]))
        r = intervals.flank(a, width=10)
        assert "score" in r.schema.names


# ---------------------------------------------------------------------------
# set_width
# ---------------------------------------------------------------------------

class TestSetWidth:
    def test_anchor_start(self):
        a = make_batch([("chr1", 10, 30)])
        r = intervals.set_width(a, width=5, anchor="start")
        assert r["start"][0].as_py() == 10
        assert r["end"][0].as_py() == 15

    def test_anchor_end(self):
        a = make_batch([("chr1", 10, 30)])
        r = intervals.set_width(a, width=5, anchor="end")
        assert r["start"][0].as_py() == 25
        assert r["end"][0].as_py() == 30

    def test_anchor_center(self):
        a = make_batch([("chr1", 10, 30)])  # midpoint = 20
        r = intervals.set_width(a, width=6, anchor="center")
        assert r["end"][0].as_py() - r["start"][0].as_py() == 6

    def test_invalid_anchor(self):
        a = make_batch([("chr1", 10, 30)])
        with pytest.raises(Exception):
            intervals.set_width(a, width=5, anchor="bogus")


# ---------------------------------------------------------------------------
# tile
# ---------------------------------------------------------------------------

class TestTile:
    def test_basic(self):
        a = make_batch([("chr1", 0, 100)])
        r = intervals.tile(a, width=30)
        assert r.num_rows == 4
        assert r["start"][0].as_py() == 0
        assert r["end"][0].as_py() == 30
        assert r["start"][3].as_py() == 90
        assert r["end"][3].as_py() == 100

    def test_exact_division(self):
        a = make_batch([("chr1", 0, 60)])
        r = intervals.tile(a, width=20)
        assert r.num_rows == 3

    def test_preserves_other_columns(self):
        a = make_batch([("chr1", 0, 40)], score=pa.array([9]))
        r = intervals.tile(a, width=20)
        assert r.num_rows == 2
        assert all(r["score"][i].as_py() == 9 for i in range(2))

    def test_multiple_rows(self):
        a = make_batch([("chr1", 0, 10), ("chr2", 0, 30)])
        r = intervals.tile(a, width=10)
        assert r.num_rows == 4

    def test_empty(self):
        a = make_batch([])
        r = intervals.tile(a, width=10)
        assert r.num_rows == 0


# ---------------------------------------------------------------------------
# disjoin
# ---------------------------------------------------------------------------

class TestDisjoin:
    def test_basic(self):
        a = make_batch([("chr1", 1, 10), ("chr1", 5, 15)])
        r = intervals.disjoin(a)
        pairs = [(r["start"][i].as_py(), r["end"][i].as_py()) for i in range(r.num_rows)]
        assert (1, 5) in pairs
        assert (5, 10) in pairs
        assert (10, 15) in pairs

    def test_no_overlap(self):
        a = make_batch([("chr1", 0, 5), ("chr1", 10, 15)])
        r = intervals.disjoin(a)
        assert r.num_rows == 2

    def test_output_schema(self):
        a = make_batch([("chr1", 0, 10)])
        r = intervals.disjoin(a)
        assert set(r.schema.names) == {"chrom", "start", "end"}

    def test_cross_chrom(self):
        a = make_batch([("chr1", 0, 10), ("chr2", 0, 10)])
        r = intervals.disjoin(a)
        assert r.num_rows == 2

    def test_three_way_overlap(self):
        a = make_batch([("chr1", 1, 10), ("chr1", 5, 15), ("chr1", 8, 20)])
        r = intervals.disjoin(a)
        # boundaries: 1,5,8,10,15,20 → pieces: [1,5],[5,8],[8,10],[10,15],[15,20]
        assert r.num_rows == 5
