import pytest
from ladle.io.vcf import Header, Reader, Record, Writer

# ---------------------------------------------------------------------------
# Minimal VCF text used across tests
# ---------------------------------------------------------------------------
HEADER_TEXT = (
    "##fileformat=VCFv4.2\n"
    "##contig=<ID=chr1,length=248956422>\n"
    "##contig=<ID=chr2,length=242193529>\n"
    "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total depth\">\n"
    "##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele frequency\">\n"
    "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
)

RECORD_LINE = "chr1\t100\trs1\tA\tG\t50.0\tPASS\tDP=30;AF=0.5\n"

VCF_TEXT = HEADER_TEXT + RECORD_LINE


# ---------------------------------------------------------------------------
# Header
# ---------------------------------------------------------------------------
class TestHeader:
    def test_parse(self):
        h = Header.parse(HEADER_TEXT)
        contigs = h.contigs()
        assert "chr1" in contigs
        assert "chr2" in contigs
        assert contigs["chr1"] == 248956422
        assert contigs["chr2"] == 242193529

    def test_empty(self):
        h = Header()
        assert h.contigs() == {}
        assert h.sample_names() == []

    def test_repr(self):
        h = Header.parse(HEADER_TEXT)
        r = repr(h)
        assert "0" in r   # 0 samples
        assert "2" in r   # 2 contigs

    def test_str_roundtrip(self):
        h = Header.parse(HEADER_TEXT)
        text = str(h)
        assert "##fileformat=" in text
        assert "chr1" in text

    def test_parse_invalid_raises(self):
        with pytest.raises((ValueError, OSError)):
            Header.parse("not a valid vcf header\n")

    def test_sample_names_empty(self):
        h = Header.parse(HEADER_TEXT)
        assert h.sample_names() == []

    def test_sample_names_with_samples(self):
        header_with_samples = (
            "##fileformat=VCFv4.2\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tNA001\tNA002\n"
        )
        h = Header.parse(header_with_samples)
        names = h.sample_names()
        assert "NA001" in names
        assert "NA002" in names

    def test_info_fields_keys(self):
        h = Header.parse(HEADER_TEXT)
        keys = [k for k, _ in h.info_fields()]
        assert "DP" in keys
        assert "AF" in keys

    def test_info_fields_types(self):
        h = Header.parse(HEADER_TEXT)
        d = dict(h.info_fields())
        assert d["DP"] == "Integer"
        assert d["AF"] == "Float"

    def test_format_fields_empty_when_no_format(self):
        h = Header.parse(HEADER_TEXT)
        assert h.format_fields() == []

    def test_format_fields_with_formats(self):
        hdr_text = (
            "##fileformat=VCFv4.2\n"
            "##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">\n"
            "##FORMAT=<ID=GQ,Number=1,Type=Integer,Description=\"GQ\">\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\n"
        )
        h = Header.parse(hdr_text)
        d = dict(h.format_fields())
        assert d["GT"] == "String"
        assert d["GQ"] == "Integer"

    def test_info_fields_flag_type(self):
        hdr_text = (
            "##fileformat=VCFv4.2\n"
            "##INFO=<ID=DB,Number=0,Type=Flag,Description=\"dbSNP\">\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
        )
        h = Header.parse(hdr_text)
        d = dict(h.info_fields())
        assert d["DB"] == "Flag"

    def test_info_fields_empty(self):
        h = Header.parse(
            "##fileformat=VCFv4.2\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
        )
        assert h.info_fields() == []


# ---------------------------------------------------------------------------
# Reader / Writer round-trip
# ---------------------------------------------------------------------------
class TestReaderWriter:
    def test_round_trip_header_only(self, tmp_path):
        path = str(tmp_path / "out.vcf")
        header = Header.parse(HEADER_TEXT)

        with Writer.from_path(path) as w:
            w.write_header(header)

        with Reader.from_path(path) as r:
            h = r.read_header()
            contigs = h.contigs()
            assert "chr1" in contigs

    def test_full_round_trip(self, tmp_path):
        path_in = str(tmp_path / "in.vcf")
        path_out = str(tmp_path / "out.vcf")

        with open(path_in, "w") as f:
            f.write(VCF_TEXT)

        with Reader.from_path(path_in) as r:
            header = r.read_header()
            records = list(r)

        assert len(records) == 1
        rec = records[0]
        assert rec.reference_sequence_name() == "chr1"
        assert rec.variant_start().get() == 100
        assert rec.reference_bases() == "A"

        with Writer.from_path(path_out) as w:
            w.write_header(header)
            w.write_record(header, rec)

        with Reader.from_path(path_out) as r:
            r.read_header()
            records2 = list(r)

        assert len(records2) == 1
        assert records2[0].reference_sequence_name() == "chr1"

    def test_multiple_records(self, tmp_path):
        records_text = "".join(
            f"chr1\t{100 + i}\t.\tA\tG\t.\t.\t.\n" for i in range(5)
        )
        path = str(tmp_path / "multi.vcf")
        with open(path, "w") as f:
            f.write(HEADER_TEXT + records_text)
        with Reader.from_path(path) as r:
            r.read_header()
            records = list(r)
        assert len(records) == 5
        assert records[0].variant_start().get() == 100
        assert records[4].variant_start().get() == 104

    def test_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.vcf")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
        with Reader.from_path(path) as r:
            r.read_header()
        assert "closed" in repr(r)

    def test_file_not_found(self):
        with pytest.raises(OSError):
            Reader.from_path("/nonexistent/file.vcf")

    def test_writer_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.vcf")
        header = Header.parse(HEADER_TEXT)
        with Writer.from_path(path) as w:
            w.write_header(header)
        assert "closed" in repr(w)


# ---------------------------------------------------------------------------
# Record fields
# ---------------------------------------------------------------------------
class TestRecord:
    @pytest.fixture
    def header_and_record(self, tmp_path):
        path = str(tmp_path / "r.vcf")
        with open(path, "w") as f:
            f.write(VCF_TEXT)
        with Reader.from_path(path) as r:
            header = r.read_header()
            record = next(iter(r))
        return header, record

    def test_reference_sequence_name(self, header_and_record):
        _, rec = header_and_record
        assert rec.reference_sequence_name() == "chr1"

    def test_variant_start(self, header_and_record):
        _, rec = header_and_record
        pos = rec.variant_start()
        assert pos is not None
        assert pos.get() == 100

    def test_ids(self, header_and_record):
        _, rec = header_and_record
        ids = rec.ids()
        assert isinstance(ids, list)
        assert "rs1" in ids

    def test_reference_bases(self, header_and_record):
        _, rec = header_and_record
        assert rec.reference_bases() == "A"

    def test_alternate_bases(self, header_and_record):
        _, rec = header_and_record
        alts = rec.alternate_bases()
        assert isinstance(alts, list)
        assert "G" in alts

    def test_quality_score(self, header_and_record):
        _, rec = header_and_record
        q = rec.quality_score()
        assert q is not None
        assert abs(q - 50.0) < 0.01

    def test_filters_pass(self, header_and_record):
        header, rec = header_and_record
        filters = rec.filters(header)
        assert isinstance(filters, list)
        assert "PASS" in filters

    def test_info(self, header_and_record):
        header, rec = header_and_record
        info = rec.info(header)
        assert isinstance(info, dict)
        assert "DP" in info
        assert info["DP"] == 30

    def test_info_float(self, header_and_record):
        header, rec = header_and_record
        info = rec.info(header)
        assert "AF" in info
        # AF is Number=A (one per alt allele), so noodles returns it as an array
        af = info["AF"]
        if isinstance(af, list):
            assert abs(af[0] - 0.5) < 0.001
        else:
            assert abs(af - 0.5) < 0.001

    def test_repr(self, header_and_record):
        _, rec = header_and_record
        r = repr(rec)
        assert "chr1" in r
        assert "100" in r

    def test_missing_quality_score(self, tmp_path):
        path = str(tmp_path / "nq.vcf")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
            f.write("chr1\t200\t.\tA\tG\t.\t.\t.\n")
        with Reader.from_path(path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.quality_score() is None

    def test_missing_ids(self, tmp_path):
        path = str(tmp_path / "noid.vcf")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
            f.write("chr1\t200\t.\tA\tG\t.\t.\t.\n")
        with Reader.from_path(path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.ids() == []

    def test_no_alt(self, tmp_path):
        path = str(tmp_path / "noalt.vcf")
        with open(path, "w") as f:
            f.write(HEADER_TEXT)
            f.write("chr1\t200\t.\tA\t.\t.\t.\t.\n")
        with Reader.from_path(path) as r:
            r.read_header()
            rec = next(iter(r))
        alts = rec.alternate_bases()
        assert alts == []

    def test_flag_info(self, tmp_path):
        header_text = (
            "##fileformat=VCFv4.2\n"
            "##INFO=<ID=DB,Number=0,Type=Flag,Description=\"dbSNP membership\">\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
        )
        path = str(tmp_path / "flag.vcf")
        with open(path, "w") as f:
            f.write(header_text)
            f.write("chr1\t100\t.\tA\tG\t.\t.\tDB\n")
        with Reader.from_path(path) as r:
            header = r.read_header()
            rec = next(iter(r))
        info = rec.info(header)
        assert "DB" in info
        assert info["DB"] is True


# ---------------------------------------------------------------------------
# Reader.from_fd
# ---------------------------------------------------------------------------


class TestReaderFromFd:
    def test_from_fd_reads_records(self, tmp_path):
        path = str(tmp_path / "t.vcf")
        with open(path, "w") as f:
            f.write(VCF_TEXT)
        fh = open(path, "rb")
        r = Reader.from_fd(fh)
        r.read_header()
        records = list(r)
        assert len(records) == 1
        assert records[0].reference_sequence_name() == "chr1"

    def test_from_fd_context_manager(self, tmp_path):
        path = str(tmp_path / "t.vcf")
        with open(path, "w") as f:
            f.write(VCF_TEXT)
        fh = open(path, "rb")
        with Reader.from_fd(fh) as r:
            r.read_header()
            records = list(r)
        assert len(records) == 1

    def test_from_fd_no_fileno_raises(self):
        import io
        with pytest.raises(OSError):
            Reader.from_fd(io.BytesIO(b"not a real file"))
