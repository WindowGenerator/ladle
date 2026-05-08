import pytest
from ladle.io.sam import Flags, Header, MappingQuality, Reader, Record, Writer

# ---------------------------------------------------------------------------
# Minimal SAM text used across tests
# ---------------------------------------------------------------------------
HEADER_TEXT = "@HD\tVN:1.6\tSO:coordinate\n@SQ\tSN:chr1\tLN:248956422\n@SQ\tSN:chr2\tLN:242193529\n"

RECORD_LINE = (
    "read1\t0\tchr1\t100\t60\t5M\t*\t0\t0\tACGTA\tIIIII\tNM:i:0\tAS:i:100\n"
)

SAM_TEXT = HEADER_TEXT + RECORD_LINE


# ---------------------------------------------------------------------------
# Flags
# ---------------------------------------------------------------------------
class TestFlags:
    def test_basic(self):
        f = Flags(0)
        assert f.bits() == 0
        assert int(f) == 0

    def test_unmapped(self):
        f = Flags(0x0004)
        assert f.is_unmapped()
        assert not f.is_segmented()

    def test_all_predicates(self):
        cases = [
            (Flags.SEGMENTED, "is_segmented"),
            (Flags.PROPERLY_SEGMENTED, "is_properly_segmented"),
            (Flags.UNMAPPED, "is_unmapped"),
            (Flags.MATE_UNMAPPED, "is_mate_unmapped"),
            (Flags.REVERSE_COMPLEMENTED, "is_reverse_complemented"),
            (Flags.MATE_REVERSE_COMPLEMENTED, "is_mate_reverse_complemented"),
            (Flags.FIRST_SEGMENT, "is_first_segment"),
            (Flags.LAST_SEGMENT, "is_last_segment"),
            (Flags.SECONDARY, "is_secondary"),
            (Flags.QC_FAIL, "is_qc_fail"),
            (Flags.DUPLICATE, "is_duplicate"),
            (Flags.SUPPLEMENTARY, "is_supplementary"),
        ]
        for bit, method in cases:
            f = Flags(bit)
            assert getattr(f, method)(), f"{method} failed for bit {bit:#06x}"

    def test_equality(self):
        assert Flags(4) == Flags(4)
        assert Flags(4) != Flags(8)

    def test_hash(self):
        assert hash(Flags(4)) == hash(Flags(4))

    def test_repr(self):
        assert "0x0004" in repr(Flags(4))

    def test_bitwise_and(self):
        assert (Flags(0x0007) & Flags(0x0004)) == Flags(0x0004)

    def test_bitwise_or(self):
        assert (Flags(0x0001) | Flags(0x0004)) == Flags(0x0005)

    def test_combined_flags(self):
        f = Flags(Flags.FIRST_SEGMENT | Flags.SEGMENTED)
        assert f.is_first_segment()
        assert f.is_segmented()
        assert not f.is_unmapped()


# ---------------------------------------------------------------------------
# MappingQuality
# ---------------------------------------------------------------------------
class TestMappingQuality:
    def test_basic(self):
        mq = MappingQuality(60)
        assert mq.get() == 60
        assert int(mq) == 60
        assert str(mq) == "60"

    def test_zero(self):
        assert MappingQuality(0).get() == 0

    def test_max_valid(self):
        assert MappingQuality(254).get() == 254

    def test_255_raises(self):
        with pytest.raises(ValueError):
            MappingQuality(255)

    def test_min_max(self):
        assert MappingQuality.MIN.get() == 0
        assert MappingQuality.MAX.get() == 254

    def test_ordering(self):
        assert MappingQuality(10) < MappingQuality(20)
        assert MappingQuality(20) > MappingQuality(10)
        assert MappingQuality(10) <= MappingQuality(10)
        assert MappingQuality(10) >= MappingQuality(10)

    def test_equality(self):
        assert MappingQuality(60) == MappingQuality(60)
        assert MappingQuality(60) != MappingQuality(61)

    def test_repr(self):
        assert "60" in repr(MappingQuality(60))


# ---------------------------------------------------------------------------
# Header
# ---------------------------------------------------------------------------
class TestHeader:
    def test_parse(self):
        h = Header.parse(HEADER_TEXT)
        seqs = h.reference_sequences()
        assert b"chr1" in seqs
        assert b"chr2" in seqs
        assert seqs[b"chr1"] == 248956422
        assert seqs[b"chr2"] == 242193529

    def test_empty(self):
        h = Header()
        assert h.reference_sequences() == {}

    def test_repr(self):
        h = Header.parse(HEADER_TEXT)
        assert "2" in repr(h)

    def test_str_roundtrip(self):
        h = Header.parse(HEADER_TEXT)
        text = str(h)
        assert "@SQ" in text
        assert "chr1" in text

    def test_parse_invalid_raises(self):
        with pytest.raises((ValueError, OSError)):
            Header.parse("not a valid sam header\n")


# ---------------------------------------------------------------------------
# Reader / Writer round-trip
# ---------------------------------------------------------------------------
class TestReaderWriter:
    def test_round_trip(self, tmp_path):
        path = str(tmp_path / "out.sam")
        header = Header.parse(HEADER_TEXT)

        # Write
        with Writer.from_path(path) as w:
            w.write_header(header)

        # Read back header
        with Reader.from_path(path) as r:
            h = r.read_header()
            seqs = h.reference_sequences()
            assert b"chr1" in seqs

    def test_full_round_trip(self, tmp_path):
        """Write a record and read it back."""
        path_in = str(tmp_path / "in.sam")
        path_out = str(tmp_path / "out.sam")

        # Write SAM with one record
        with open(path_in, "w") as f:
            f.write(SAM_TEXT)

        # Read it
        with Reader.from_path(path_in) as r:
            header = r.read_header()
            records = list(r)

        assert len(records) == 1
        rec = records[0]
        assert rec.name() == b"read1"
        assert rec.flags().bits() == 0
        assert rec.reference_sequence_name() == b"chr1"
        assert rec.alignment_start().get() == 100
        assert rec.mapping_quality().get() == 60
        assert rec.cigar() == b"5M"
        assert rec.sequence() == b"ACGTA"
        assert rec.quality_scores() == b"IIIII"

        # Write it back
        with Writer.from_path(path_out) as w:
            w.write_header(header)
            w.write_record(header, rec)

        # Re-read
        with Reader.from_path(path_out) as r:
            r.read_header()
            records2 = list(r)

        assert len(records2) == 1
        assert records2[0].name() == b"read1"

    def test_read_header_before_iteration(self, tmp_path):
        path = str(tmp_path / "t.sam")
        with open(path, "w") as f:
            f.write(SAM_TEXT)
        with Reader.from_path(path) as r:
            h = r.read_header()
            assert b"chr1" in h.reference_sequences()
            records = list(r)
        assert len(records) == 1

    def test_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.sam")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
        with Reader.from_path(path) as r:
            r.read_header()
        assert "closed" in repr(r)

    def test_file_not_found(self):
        with pytest.raises(OSError):
            Reader.from_path("/nonexistent/file.sam")

    def test_multiple_records(self, tmp_path):
        records_text = "".join(
            f"read{i}\t0\tchr1\t{100 + i}\t60\t5M\t*\t0\t0\tACGTA\tIIIII\n"
            for i in range(5)
        )
        path = str(tmp_path / "multi.sam")
        with open(path, "w") as f:
            f.write(HEADER_TEXT + records_text)
        with Reader.from_path(path) as r:
            r.read_header()
            records = list(r)
        assert len(records) == 5
        assert records[0].name() == b"read0"
        assert records[4].name() == b"read4"


# ---------------------------------------------------------------------------
# Record fields
# ---------------------------------------------------------------------------
class TestRecord:
    @pytest.fixture
    def record(self, tmp_path):
        path = str(tmp_path / "r.sam")
        with open(path, "w") as f:
            f.write(SAM_TEXT)
        with Reader.from_path(path) as r:
            r.read_header()
            return next(iter(r))

    def test_name(self, record):
        assert record.name() == b"read1"

    def test_flags(self, record):
        f = record.flags()
        assert isinstance(f, Flags)
        assert f.bits() == 0

    def test_reference_sequence_name(self, record):
        assert record.reference_sequence_name() == b"chr1"

    def test_alignment_start(self, record):
        pos = record.alignment_start()
        assert pos is not None
        assert pos.get() == 100

    def test_mapping_quality(self, record):
        mq = record.mapping_quality()
        assert mq is not None
        assert mq.get() == 60

    def test_cigar(self, record):
        assert record.cigar() == b"5M"

    def test_mate_fields(self, record):
        assert record.mate_reference_sequence_name() is None
        assert record.mate_alignment_start() is None

    def test_template_length(self, record):
        assert record.template_length() == 0

    def test_sequence(self, record):
        assert record.sequence() == b"ACGTA"

    def test_quality_scores(self, record):
        assert record.quality_scores() == b"IIIII"

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
        path = str(tmp_path / "u.sam")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
            f.write("read_unmapped\t4\t*\t0\t0\t*\t*\t0\t0\tACGT\tIIII\n")
        with Reader.from_path(path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.flags().is_unmapped()
        assert rec.reference_sequence_name() is None
        assert rec.alignment_start() is None
        # MAPQ 0 is valid (not missing); MAPQ 255 = missing/unavailable → None
        mq = rec.mapping_quality()
        assert mq is None or mq.get() == 0
