import os
import tempfile

import pytest

from ladle.io.bam import Reader, RecordBatch, RecordBatchIterator, Record, Writer
from ladle.io.sam import Header, Reader as SamReader

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

HEADER_TEXT = (
    "@HD\tVN:1.6\tSO:coordinate\n"
    "@SQ\tSN:chr1\tLN:248956422\n"
    "@SQ\tSN:chr2\tLN:242193529\n"
)

RECORDS_TEXT = "".join(
    f"read{i}\t0\tchr1\t{100 + i}\t60\t5M\t*\t0\t0\tACGTA\tIIIII\n"
    for i in range(5)
)

SAM_TEXT = HEADER_TEXT + RECORDS_TEXT

UNMAPPED_TEXT = HEADER_TEXT + "unmapped\t4\t*\t0\t0\t*\t*\t0\t0\tACGTA\tIIIII\n"


def _write_bam(sam_text: str, bam_path: str) -> Header:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".sam", delete=False) as f:
        f.write(sam_text)
        tmp = f.name
    try:
        with SamReader.from_path(tmp) as r:
            header = r.read_header()
            sam_records = list(r)
        with Writer.from_path(bam_path) as w:
            w.write_header(header)
            for rec in sam_records:
                w.write_sam_record(header, rec)
        return header
    finally:
        os.unlink(tmp)


@pytest.fixture
def bam_path(tmp_path):
    path = str(tmp_path / "test.bam")
    _write_bam(SAM_TEXT, path)
    return path


@pytest.fixture
def batch(bam_path):
    with Reader.from_path(bam_path) as r:
        r.read_header()
        return r.records_to_batch()


# ---------------------------------------------------------------------------
# Module-level checks
# ---------------------------------------------------------------------------

class TestImports:
    def test_record_batch_class_exists(self):
        assert RecordBatch

    def test_record_batch_iterator_class_exists(self):
        assert RecordBatchIterator

    def test_recordbatch_accessible_from_io_bam(self):
        import ladle.io.bam as io_bam
        assert io_bam.RecordBatch is RecordBatch


# ---------------------------------------------------------------------------
# RecordBatch basic
# ---------------------------------------------------------------------------

class TestRecordBatch:
    def test_type(self, batch):
        assert isinstance(batch, RecordBatch)

    def test_len(self, batch):
        assert len(batch) == 5

    def test_repr(self, batch):
        r = repr(batch)
        assert "RecordBatch" in r
        assert "5" in r

    def test_empty_batch(self, tmp_path):
        path = str(tmp_path / "empty.bam")
        _write_bam(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert len(b) == 0

    def test_records_to_batch_requires_read_header(self, tmp_path):
        path = str(tmp_path / "t.bam")
        _write_bam(SAM_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert len(b) == 5


# ---------------------------------------------------------------------------
# to_arrow
# ---------------------------------------------------------------------------

class TestToArrow:
    def test_returns_record_batch(self, batch):
        pa = pytest.importorskip("pyarrow")
        b = batch.to_arrow()
        assert isinstance(b, pa.RecordBatch)

    def test_num_rows(self, batch):
        pytest.importorskip("pyarrow")
        assert batch.to_arrow().num_rows == 5

    def test_schema_field_names(self, batch):
        pytest.importorskip("pyarrow")
        names = batch.to_arrow().schema.names
        for field in ("name", "flags", "reference_sequence_id", "alignment_start",
                      "mapping_quality", "cigar", "mate_reference_sequence_id",
                      "mate_alignment_start", "template_length", "sequence",
                      "quality_scores"):
            assert field in names

    def test_flags_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        schema = batch.to_arrow().schema
        assert schema.field("flags").type == pa.uint16()

    def test_alignment_start_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("alignment_start").type == pa.int32()

    def test_name_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("name").type == pa.large_binary()

    def test_mapping_quality_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("mapping_quality").type == pa.uint8()

    def test_sequence_values(self, batch):
        pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("sequence")
        # BAM sequence is stored 4-bit packed; all 5 records have same sequence
        seq = col[0].as_py()
        assert isinstance(seq, bytes)
        assert len(seq) > 0

    def test_name_values(self, batch):
        pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("name")
        assert col[0].as_py() == b"read0"

    def test_alignment_start_values(self, batch):
        pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("alignment_start")
        assert col[0].as_py() == 100
        assert col[4].as_py() == 104

    def test_empty_batch_arrow(self, tmp_path):
        pytest.importorskip("pyarrow")
        path = str(tmp_path / "empty.bam")
        _write_bam(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert b.to_arrow().num_rows == 0

    def test_unmapped_null_alignment_start(self, tmp_path):
        pytest.importorskip("pyarrow")
        path = str(tmp_path / "unmap.bam")
        _write_bam(UNMAPPED_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        col = b.to_arrow().column("alignment_start")
        assert col[0].as_py() is None

    def test_nullable_fields_declared_nullable(self, batch):
        pytest.importorskip("pyarrow")
        schema = batch.to_arrow().schema
        for nullable_field in ("name", "reference_sequence_id", "alignment_start",
                               "mapping_quality", "mate_reference_sequence_id",
                               "mate_alignment_start"):
            assert schema.field(nullable_field).nullable


# ---------------------------------------------------------------------------
# to_iterator
# ---------------------------------------------------------------------------

class TestToIterator:
    def test_yields_records(self, batch):
        records = list(batch.to_iterator())
        assert len(records) == 5

    def test_yields_bam_record_type(self, batch):
        for rec in batch.to_iterator():
            assert isinstance(rec, Record)

    def test_record_fields_accessible(self, batch):
        rec = next(iter(batch.to_iterator()))
        assert rec.name() == b"read0"
        # sequence is 4-bit BAM-encoded; just verify it's non-empty bytes
        assert isinstance(rec.sequence(), bytes)
        assert len(rec.sequence()) > 0

    def test_iterator_protocol(self, batch):
        it = batch.to_iterator()
        assert iter(it) is it
        first = next(it)
        assert isinstance(first, Record)

    def test_empty_batch_iterator(self, tmp_path):
        path = str(tmp_path / "empty.bam")
        _write_bam(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert list(b.to_iterator()) == []


# ---------------------------------------------------------------------------
# to_polars
# ---------------------------------------------------------------------------

class TestToPolars:
    def test_returns_dataframe(self, batch):
        pl = pytest.importorskip("polars")
        df = batch.to_polars()
        assert isinstance(df, pl.DataFrame)

    def test_row_count(self, batch):
        pytest.importorskip("polars")
        assert batch.to_polars().height == 5

    def test_column_names(self, batch):
        pytest.importorskip("polars")
        cols = batch.to_polars().columns
        assert "flags" in cols
        assert "sequence" in cols


# ---------------------------------------------------------------------------
# to_pandas
# ---------------------------------------------------------------------------

class TestToPandas:
    def test_returns_dataframe(self, batch):
        pytest.importorskip("pyarrow")
        pd = pytest.importorskip("pandas")
        df = batch.to_pandas()
        assert isinstance(df, pd.DataFrame)

    def test_row_count(self, batch):
        pytest.importorskip("pyarrow")
        pytest.importorskip("pandas")
        assert len(batch.to_pandas()) == 5


# ---------------------------------------------------------------------------
# from_arrow
# ---------------------------------------------------------------------------

class TestFromArrow:
    def test_round_trip_len(self, batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(batch.to_arrow())
        assert len(b2) == 5

    def test_round_trip_values(self, batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(batch.to_arrow())
        col = b2.to_arrow().column("name")
        assert col[0].as_py() == b"read0"

    def test_round_trip_to_polars(self, batch):
        pl = pytest.importorskip("polars")
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(batch.to_arrow())
        df = b2.to_polars()
        assert isinstance(df, pl.DataFrame)
        assert df.height == 5

    def test_to_iterator_raises(self, batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(batch.to_arrow())
        with pytest.raises(OSError):
            b2.to_iterator()

    def test_returns_record_batch_instance(self, batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(batch.to_arrow())
        assert isinstance(b2, RecordBatch)

    def test_empty(self, tmp_path):
        pytest.importorskip("pyarrow")
        path = str(tmp_path / "empty.bam")
        _write_bam(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        b2 = RecordBatch.from_arrow(b.to_arrow())
        assert len(b2) == 0


# ---------------------------------------------------------------------------
# from_polars
# ---------------------------------------------------------------------------

class TestFromPolars:
    def test_round_trip_len(self, batch):
        pytest.importorskip("polars")
        b2 = RecordBatch.from_polars(batch.to_polars())
        assert len(b2) == 5

    def test_round_trip_values(self, batch):
        pytest.importorskip("polars")
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_polars(batch.to_polars())
        col = b2.to_arrow().column("flags")
        assert col[0].as_py() is not None

    def test_returns_record_batch_instance(self, batch):
        pytest.importorskip("polars")
        b2 = RecordBatch.from_polars(batch.to_polars())
        assert isinstance(b2, RecordBatch)

    def test_to_iterator_raises(self, batch):
        pytest.importorskip("polars")
        b2 = RecordBatch.from_polars(batch.to_polars())
        with pytest.raises(OSError):
            b2.to_iterator()


# ---------------------------------------------------------------------------
# from_pandas
# ---------------------------------------------------------------------------

class TestFromPandas:
    def test_round_trip_len(self, batch):
        pytest.importorskip("pyarrow")
        pytest.importorskip("pandas")
        b2 = RecordBatch.from_pandas(batch.to_pandas())
        assert len(b2) == 5

    def test_returns_record_batch_instance(self, batch):
        pytest.importorskip("pyarrow")
        pytest.importorskip("pandas")
        b2 = RecordBatch.from_pandas(batch.to_pandas())
        assert isinstance(b2, RecordBatch)

    def test_to_iterator_raises(self, batch):
        pytest.importorskip("pyarrow")
        pytest.importorskip("pandas")
        b2 = RecordBatch.from_pandas(batch.to_pandas())
        with pytest.raises(OSError):
            b2.to_iterator()
