import struct

import pytest

import ladle.io.core as core
from ladle.io.bam import IndexedReader, Query, Reader, Record, Writer, bai
from ladle.io.sam import Flags, Header, MappingQuality, Reader as SamReader

# ---------------------------------------------------------------------------
# Shared fixtures / helpers
# ---------------------------------------------------------------------------

HEADER_TEXT = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:248956422\n@SQ\tSN:chr2\tLN:242193529\n"

RECORD_LINE = "read1\t0\tchr1\t100\t60\t5M\t*\t0\t0\tACGTA\tIIIII\tNM:i:0\tAS:i:100\n"

SAM_TEXT = HEADER_TEXT + RECORD_LINE


def _write_bam_from_sam(sam_text: str, bam_path: str) -> Header:
    """Write a BAM file from SAM text, return the parsed header."""
    import os

    sam_path = bam_path + ".sam_tmp"
    with open(sam_path, "w") as f:
        f.write(sam_text)
    try:
        with SamReader.from_path(sam_path) as r:
            header = r.read_header()
            sam_records = list(r)
        with Writer.from_path(bam_path) as w:
            w.write_header(header)
            for rec in sam_records:
                w.write_sam_record(header, rec)
        return header
    finally:
        os.unlink(sam_path)


def _write_minimal_bai(bai_path: str, n_ref: int = 2) -> None:
    """Write a minimal valid BAI with no chunks (for open/header tests)."""
    body = b""
    for _ in range(n_ref):
        body += struct.pack("<i", 0)  # n_bin = 0
        body += struct.pack("<i", 0)  # n_intv = 0
    with open(bai_path, "wb") as f:
        f.write(b"BAI\x01" + struct.pack("<i", n_ref) + body)


# ---------------------------------------------------------------------------
# Imports / module structure
# ---------------------------------------------------------------------------


class TestImports:
    def test_classes_exist(self):
        assert Reader
        assert Writer
        assert IndexedReader
        assert Query
        assert Record
        assert bai.Index

    def test_bai_submodule(self):
        import ladle.io.bam as bam_mod

        assert hasattr(bam_mod, "bai")
        assert hasattr(bam_mod.bai, "Index")


# ---------------------------------------------------------------------------
# Reader / Writer round-trip
# ---------------------------------------------------------------------------


class TestReaderWriter:
    def test_header_only_round_trip(self, tmp_path):
        path = str(tmp_path / "header_only.bam")
        header = Header.parse(HEADER_TEXT)

        with Writer.from_path(path) as w:
            w.write_header(header)

        with Reader.from_path(path) as r:
            h2 = r.read_header()
            assert b"chr1" in h2.reference_sequences()
            assert b"chr2" in h2.reference_sequences()
            records = list(r)

        assert records == []

    def test_full_round_trip(self, tmp_path):
        bam_path = str(tmp_path / "out.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)

        with Reader.from_path(bam_path) as r:
            h2 = r.read_header()
            records = list(r)

        assert b"chr1" in h2.reference_sequences()
        assert len(records) == 1
        rec = records[0]
        assert rec.name() == b"read1"

    def test_multiple_records(self, tmp_path):
        n = 5
        lines = "".join(
            f"read{i}\t0\tchr1\t{100 + i}\t60\t5M\t*\t0\t0\tACGTA\tIIIII\n" for i in range(n)
        )
        bam_path = str(tmp_path / "multi.bam")
        _write_bam_from_sam(HEADER_TEXT + lines, bam_path)

        with Reader.from_path(bam_path) as r:
            r.read_header()
            records = list(r)

        assert len(records) == n
        assert records[0].name() == b"read0"
        assert records[-1].name() == b"read4"

    def test_context_manager_closes(self, tmp_path):
        bam_path = str(tmp_path / "t.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)
        with Reader.from_path(bam_path) as r:
            r.read_header()
        assert "closed" in repr(r)

    def test_writer_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "w.bam")
        with Writer.from_path(path) as w:
            w.write_header(Header.parse(HEADER_TEXT))
        assert "closed" in repr(w)

    def test_read_header_before_iteration(self, tmp_path):
        bam_path = str(tmp_path / "t.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)
        with Reader.from_path(bam_path) as r:
            h = r.read_header()
            assert b"chr1" in h.reference_sequences()
            records = list(r)
        assert len(records) == 1

    def test_file_not_found(self):
        with pytest.raises(OSError):
            Reader.from_path("/nonexistent/file.bam")

    def test_write_file_not_writable(self):
        with pytest.raises(OSError):
            Writer.from_path("/nonexistent/dir/out.bam")


# ---------------------------------------------------------------------------
# Record field accessors
# ---------------------------------------------------------------------------


class TestRecord:
    @pytest.fixture
    def record(self, tmp_path):
        bam_path = str(tmp_path / "r.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)
        with Reader.from_path(bam_path) as r:
            r.read_header()
            return next(iter(r))

    def test_name(self, record):
        assert record.name() == b"read1"

    def test_flags(self, record):
        f = record.flags()
        assert isinstance(f, Flags)
        assert f.bits() == 0

    def test_reference_sequence_id(self, record):
        rsid = record.reference_sequence_id()
        assert rsid == 0

    def test_alignment_start(self, record):
        pos = record.alignment_start()
        assert pos is not None
        assert isinstance(pos, core.Position)
        assert pos.get() == 100

    def test_mapping_quality(self, record):
        mq = record.mapping_quality()
        assert mq is not None
        assert isinstance(mq, MappingQuality)
        assert mq.get() == 60

    def test_template_length(self, record):
        assert record.template_length() == 0

    def test_sequence_is_bytes(self, record):
        seq = record.sequence()
        assert isinstance(seq, bytes)
        assert len(seq) > 0

    def test_quality_scores_is_bytes(self, record):
        qs = record.quality_scores()
        assert isinstance(qs, bytes)
        assert len(qs) > 0

    def test_cigar_is_bytes(self, record):
        cigar = record.cigar()
        assert isinstance(cigar, bytes)
        assert len(cigar) > 0

    def test_mate_reference_sequence_id_none(self, record):
        assert record.mate_reference_sequence_id() is None

    def test_mate_alignment_start_none(self, record):
        assert record.mate_alignment_start() is None

    def test_data(self, record):
        d = record.data()
        assert isinstance(d, dict)
        assert b"NM" in d
        assert d[b"NM"] == 0
        assert b"AS" in d
        assert d[b"AS"] == 100

    def test_repr(self, record):
        assert "read1" in repr(record)

    def test_unmapped_record(self, tmp_path):
        sam_text = HEADER_TEXT + "unmapped\t4\t*\t0\t0\t*\t*\t0\t0\tACGT\tIIII\n"
        bam_path = str(tmp_path / "u.bam")
        _write_bam_from_sam(sam_text, bam_path)
        with Reader.from_path(bam_path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.flags().is_unmapped()
        assert rec.reference_sequence_id() is None
        assert rec.alignment_start() is None


# ---------------------------------------------------------------------------
# BAI Index
# ---------------------------------------------------------------------------


class TestBaiIndex:
    def test_read_minimal_bai(self, tmp_path):
        bai_path = str(tmp_path / "sample.bai")
        _write_minimal_bai(bai_path, n_ref=2)
        idx = bai.Index.read_from_path(bai_path)
        assert idx is not None
        assert "bai" in repr(idx).lower()

    def test_read_nonexistent(self):
        with pytest.raises(OSError):
            bai.Index.read_from_path("/nonexistent/file.bai")

    def test_repr(self, tmp_path):
        bai_path = str(tmp_path / "sample.bai")
        _write_minimal_bai(bai_path, n_ref=1)
        idx = bai.Index.read_from_path(bai_path)
        r = repr(idx)
        assert "Index" in r


# ---------------------------------------------------------------------------
# IndexedReader
# ---------------------------------------------------------------------------


class TestIndexedReader:
    @pytest.fixture
    def indexed_bam(self, tmp_path):
        """Create a BAM + minimal BAI pair. Returns (bam_path, header)."""
        bam_path = str(tmp_path / "sample.bam")
        bai_path = bam_path + ".bai"
        header = _write_bam_from_sam(SAM_TEXT, bam_path)
        _write_minimal_bai(bai_path, n_ref=2)
        return bam_path, header

    def test_open_and_read_header(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
        assert b"chr1" in h.reference_sequences()

    def test_context_manager_closes(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            r.read_header()
        assert "closed" in repr(r)

    def test_file_not_found(self):
        with pytest.raises(OSError):
            IndexedReader.from_path("/nonexistent/file.bam")

    def test_query_returns_query_object(self, indexed_bam):
        bam_path, header = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
            region = core.Region.parse("chr1:1-248956422")
            q = r.query(h, region)
        assert isinstance(q, Query)

    def test_query_is_iterable(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
            region = core.Region.parse("chr1:1-248956422")
            q = r.query(h, region)
            results = list(q)
        assert isinstance(results, list)

    def test_query_len(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
            region = core.Region.parse("chr1:1-248956422")
            q = r.query(h, region)
        assert isinstance(len(q), int)

    def test_query_repr(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
            region = core.Region.parse("chr1:1-248956422")
            q = r.query(h, region)
        assert "Query" in repr(q)

    def test_query_records_are_bam_records(self, indexed_bam):
        bam_path, _ = indexed_bam
        with IndexedReader.from_path(bam_path) as r:
            h = r.read_header()
            region = core.Region.parse("chr1:1-248956422")
            q = r.query(h, region)
            for rec in q:
                assert isinstance(rec, Record)


# ---------------------------------------------------------------------------
# Reader.from_fd
# ---------------------------------------------------------------------------


class TestReaderFromFd:
    def test_from_fd_reads_records(self, tmp_path):
        bam_path = str(tmp_path / "t.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)
        f = open(bam_path, "rb")
        r = Reader.from_fd(f)
        r.read_header()
        records = list(r)
        assert len(records) == 1
        assert records[0].name() == b"read1"

    def test_from_fd_context_manager(self, tmp_path):
        bam_path = str(tmp_path / "t.bam")
        _write_bam_from_sam(SAM_TEXT, bam_path)
        f = open(bam_path, "rb")
        with Reader.from_fd(f) as r:
            r.read_header()
            records = list(r)
        assert len(records) == 1

    def test_from_fd_no_fileno_raises(self):
        import io

        with pytest.raises(OSError):
            Reader.from_fd(io.BytesIO(b"not a real file"))
