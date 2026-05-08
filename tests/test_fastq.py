import io
import pytest
from ladle.io.fastq import Reader, Writer

# ---------------------------------------------------------------------------
# Minimal FASTQ text used across tests
# ---------------------------------------------------------------------------
RECORD_1 = "@read1\nACGT\n+\nIIII\n"
RECORD_2 = "@read2 some description\nTGCA\n+\nHHHH\n"
FASTQ_TEXT = RECORD_1 + RECORD_2


def _write_fastq(path: str, text: str) -> None:
    with open(path, "w") as f:
        f.write(text)


# ---------------------------------------------------------------------------
# Reader
# ---------------------------------------------------------------------------
class TestReader:
    def test_from_path_reads_records(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        with Reader.from_path(path) as r:
            records = list(r)
        assert len(records) == 2

    def test_name(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_1)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert rec.name() == b"read1"

    def test_sequence(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_1)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert rec.sequence() == b"ACGT"

    def test_quality_scores(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_1)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert rec.quality_scores() == b"IIII"

    def test_description_empty(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_1)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert rec.description() is None

    def test_description_non_empty(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_2)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert rec.description() == b"some description"

    def test_multiple_records(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        with Reader.from_path(path) as r:
            records = list(r)
        assert records[0].name() == b"read1"
        assert records[1].name() == b"read2"

    def test_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        with Reader.from_path(path) as r:
            list(r)
        assert "closed" in repr(r)

    def test_file_not_found(self):
        with pytest.raises(OSError):
            Reader.from_path("/nonexistent/file.fastq")

    def test_repr_open(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        r = Reader.from_path(path)
        assert "open" in repr(r)
        r.close()

    def test_repr_closed(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        r = Reader.from_path(path)
        r.close()
        assert "closed" in repr(r)


# ---------------------------------------------------------------------------
# Record repr
# ---------------------------------------------------------------------------
class TestRecord:
    def test_repr(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, RECORD_1)
        with Reader.from_path(path) as r:
            rec = next(iter(r))
        assert "read1" in repr(rec)


# ---------------------------------------------------------------------------
# Writer
# ---------------------------------------------------------------------------
class TestWriter:
    def test_round_trip(self, tmp_path):
        src = str(tmp_path / "in.fastq")
        dst = str(tmp_path / "out.fastq")
        _write_fastq(src, FASTQ_TEXT)

        with Reader.from_path(src) as r:
            records = list(r)

        with Writer.from_path(dst) as w:
            for rec in records:
                w.write_record(rec)

        with Reader.from_path(dst) as r:
            out = list(r)

        assert len(out) == 2
        assert out[0].name() == b"read1"
        assert out[0].sequence() == b"ACGT"
        assert out[1].name() == b"read2"

    def test_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        with Writer.from_path(path) as w:
            pass
        assert "closed" in repr(w)

    def test_repr_open(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        w = Writer.from_path(path)
        assert "open" in repr(w)
        w.close()


# ---------------------------------------------------------------------------
# from_fd
# ---------------------------------------------------------------------------
class TestReaderFromFd:
    def test_from_fd_reads_records(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        fh = open(path, "rb")
        r = Reader.from_fd(fh)
        records = list(r)
        fh.close()
        assert len(records) == 2
        assert records[0].name() == b"read1"

    def test_from_fd_context_manager(self, tmp_path):
        path = str(tmp_path / "t.fastq")
        _write_fastq(path, FASTQ_TEXT)
        fh = open(path, "rb")
        with Reader.from_fd(fh) as r:
            records = list(r)
        fh.close()
        assert len(records) == 2

    def test_from_fd_no_fileno_raises(self):
        with pytest.raises(OSError):
            Reader.from_fd(io.BytesIO(b"@r\nA\n+\nI\n"))
