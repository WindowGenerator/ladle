import os
import tempfile

import pytest

import ladle.io.vcf as vcf
import ladle.io.bcf as bcf
from ladle.io.bcf import RecordBatch

# ---------------------------------------------------------------------------
# Minimal VCF text used to create BCF fixtures
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
# Helper: write a VCF string as a BCF file
# ---------------------------------------------------------------------------
def _vcf_text_to_bcf(vcf_text: str, bcf_path: str) -> None:
    with tempfile.NamedTemporaryFile(mode="w", suffix=".vcf", delete=False) as f:
        f.write(vcf_text)
        tmp = f.name
    try:
        with vcf.Reader.from_path(tmp) as r:
            hdr = r.read_header()
            records = list(r)
        with bcf.Writer.from_path(bcf_path) as w:
            w.write_header(hdr)
            for rec in records:
                w.write_vcf_record(hdr, rec)
    finally:
        os.unlink(tmp)


@pytest.fixture
def bcf_path(tmp_path):
    path = str(tmp_path / "test.bcf")
    _vcf_text_to_bcf(VCF_TEXT, path)
    return path


@pytest.fixture
def header_and_record(bcf_path):
    with bcf.Reader.from_path(bcf_path) as r:
        header = r.read_header()
        record = next(iter(r))
    return header, record


# ---------------------------------------------------------------------------
# Reader / Writer
# ---------------------------------------------------------------------------
class TestReaderWriter:
    def test_round_trip_header_only(self, tmp_path):
        path = str(tmp_path / "hdr.bcf")
        hdr = vcf.Header.parse(HEADER_TEXT)
        with bcf.Writer.from_path(path) as w:
            w.write_header(hdr)
        with bcf.Reader.from_path(path) as r:
            h = r.read_header()
            contigs = h.contigs()
        assert "chr1" in contigs
        assert contigs["chr1"] == 248956422

    def test_full_round_trip(self, tmp_path):
        bcf_path = str(tmp_path / "rt.bcf")
        _vcf_text_to_bcf(VCF_TEXT, bcf_path)

        with bcf.Reader.from_path(bcf_path) as r:
            header = r.read_header()
            records = list(r)

        assert len(records) == 1
        rec = records[0]
        assert rec.reference_sequence_name(header) == "chr1"
        assert rec.variant_start().get() == 100

        bcf_path2 = str(tmp_path / "rt2.bcf")
        with bcf.Writer.from_path(bcf_path2) as w:
            w.write_header(header)
            w.write_record(header, rec)

        with bcf.Reader.from_path(bcf_path2) as r:
            header2 = r.read_header()
            records2 = list(r)

        assert len(records2) == 1
        assert records2[0].reference_sequence_name(header2) == "chr1"

    def test_multiple_records(self, tmp_path):
        records_text = "".join(
            f"chr1\t{100 + i}\t.\tA\tG\t.\t.\t.\n" for i in range(5)
        )
        bcf_path = str(tmp_path / "multi.bcf")
        _vcf_text_to_bcf(HEADER_TEXT + records_text, bcf_path)

        with bcf.Reader.from_path(bcf_path) as r:
            r.read_header()
            records = list(r)

        assert len(records) == 5
        assert records[0].variant_start().get() == 100
        assert records[4].variant_start().get() == 104

    def test_context_manager_closes(self, bcf_path):
        with bcf.Reader.from_path(bcf_path) as r:
            r.read_header()
        assert "closed" in repr(r)

    def test_writer_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "t.bcf")
        hdr = vcf.Header.parse(HEADER_TEXT)
        with bcf.Writer.from_path(path) as w:
            w.write_header(hdr)
        assert "closed" in repr(w)

    def test_file_not_found(self):
        with pytest.raises(OSError):
            bcf.Reader.from_path("/nonexistent/file.bcf")


# ---------------------------------------------------------------------------
# Record fields
# ---------------------------------------------------------------------------
class TestRecord:
    def test_reference_sequence_name(self, header_and_record):
        header, rec = header_and_record
        assert rec.reference_sequence_name(header) == "chr1"

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
        ref = rec.reference_bases()
        assert ref == b"A"

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
        af = info["AF"]
        if isinstance(af, list):
            assert abs(af[0] - 0.5) < 0.001
        else:
            assert abs(af - 0.5) < 0.001

    def test_repr(self, header_and_record):
        _, rec = header_and_record
        r = repr(rec)
        assert "100" in r

    def test_missing_quality_score(self, tmp_path):
        bcf_path = str(tmp_path / "nq.bcf")
        _vcf_text_to_bcf(HEADER_TEXT + "chr1\t200\t.\tA\tG\t.\t.\t.\n", bcf_path)
        with bcf.Reader.from_path(bcf_path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.quality_score() is None

    def test_missing_ids(self, tmp_path):
        bcf_path = str(tmp_path / "noid.bcf")
        _vcf_text_to_bcf(HEADER_TEXT + "chr1\t200\t.\tA\tG\t.\t.\t.\n", bcf_path)
        with bcf.Reader.from_path(bcf_path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.ids() == []

    def test_no_alt(self, tmp_path):
        bcf_path = str(tmp_path / "noalt.bcf")
        _vcf_text_to_bcf(HEADER_TEXT + "chr1\t200\t.\tA\t.\t.\t.\t.\n", bcf_path)
        with bcf.Reader.from_path(bcf_path) as r:
            r.read_header()
            rec = next(iter(r))
        assert rec.alternate_bases() == []

    def test_flag_info(self, tmp_path):
        header_text = (
            "##fileformat=VCFv4.2\n"
            "##contig=<ID=chr1,length=248956422>\n"
            "##INFO=<ID=DB,Number=0,Type=Flag,Description=\"dbSNP membership\">\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
        )
        bcf_path = str(tmp_path / "flag.bcf")
        _vcf_text_to_bcf(header_text + "chr1\t100\t.\tA\tG\t.\t.\tDB\n", bcf_path)
        with bcf.Reader.from_path(bcf_path) as r:
            header = r.read_header()
            rec = next(iter(r))
        info = rec.info(header)
        assert "DB" in info
        assert info["DB"] is True


# ---------------------------------------------------------------------------
# Header.info_fields / format_fields
# ---------------------------------------------------------------------------

INFO_FMT_HEADER_TEXT = (
    "##fileformat=VCFv4.2\n"
    "##contig=<ID=chr1,length=248956422>\n"
    "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total depth\">\n"
    "##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele frequency\">\n"
    "##INFO=<ID=DB,Number=0,Type=Flag,Description=\"dbSNP\">\n"
    "##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">\n"
    "##FORMAT=<ID=GQ,Number=1,Type=Integer,Description=\"Genotype quality\">\n"
    "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE1\tSAMPLE2\n"
)

INFO_FMT_RECORDS_TEXT = (
    "chr1\t100\trs1\tA\tG\t30.0\tPASS\tDP=20;AF=0.5;DB\tGT:GQ\t0/1:40\t1/1:50\n"
    "chr1\t200\trs2\tC\tT\t25.0\tPASS\tDP=15;AF=0.3\tGT:GQ\t0/0:60\t0/1:35\n"
)


@pytest.fixture
def info_fmt_bcf_path(tmp_path):
    path = str(tmp_path / "info_fmt.bcf")
    _vcf_text_to_bcf(INFO_FMT_HEADER_TEXT + INFO_FMT_RECORDS_TEXT, path)
    return path


@pytest.fixture
def info_fmt_header(info_fmt_bcf_path):
    with bcf.Reader.from_path(info_fmt_bcf_path) as r:
        return r.read_header()


class TestHeaderFields:
    def test_info_fields_returns_list(self, info_fmt_header):
        fields = info_fmt_header.info_fields()
        assert isinstance(fields, list)

    def test_info_fields_keys(self, info_fmt_header):
        keys = [k for k, _ in info_fmt_header.info_fields()]
        assert "DP" in keys
        assert "AF" in keys
        assert "DB" in keys

    def test_info_fields_types(self, info_fmt_header):
        d = dict(info_fmt_header.info_fields())
        assert d["DP"] == "Integer"
        assert d["AF"] == "Float"
        assert d["DB"] == "Flag"

    def test_format_fields_returns_list(self, info_fmt_header):
        fields = info_fmt_header.format_fields()
        assert isinstance(fields, list)

    def test_format_fields_keys(self, info_fmt_header):
        keys = [k for k, _ in info_fmt_header.format_fields()]
        assert "GT" in keys
        assert "GQ" in keys

    def test_format_fields_types(self, info_fmt_header):
        d = dict(info_fmt_header.format_fields())
        assert d["GT"] == "String"
        assert d["GQ"] == "Integer"

    def test_empty_header_info_fields(self):
        hdr = vcf.Header.parse("##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n")
        assert hdr.info_fields() == []

    def test_empty_header_format_fields(self):
        hdr = vcf.Header.parse("##fileformat=VCFv4.2\n#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n")
        assert hdr.format_fields() == []


# ---------------------------------------------------------------------------
# RecordBatch with INFO/FORMAT columns (BCF)
# ---------------------------------------------------------------------------

@pytest.fixture
def info_fmt_batch(info_fmt_bcf_path):
    with bcf.Reader.from_path(info_fmt_bcf_path) as r:
        hdr = r.read_header()
        return r.records_to_batch(hdr)


class TestBcfRecordBatch:
    def test_type(self, info_fmt_batch):
        assert isinstance(info_fmt_batch, RecordBatch)

    def test_len(self, info_fmt_batch):
        assert len(info_fmt_batch) == 2

    def test_repr(self, info_fmt_batch):
        assert "RecordBatch" in repr(info_fmt_batch)

    def test_has_base_columns(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        for col in ("chrom", "pos", "ref", "alt", "qual"):
            assert col in names

    def test_has_info_dp_column(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        assert "INFO_DP" in info_fmt_batch.to_arrow().schema.names

    def test_has_info_af_column(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        assert "INFO_AF" in info_fmt_batch.to_arrow().schema.names

    def test_has_info_db_column(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        assert "INFO_DB" in info_fmt_batch.to_arrow().schema.names

    def test_has_fmt_gt_column(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        assert "FMT_GT" in info_fmt_batch.to_arrow().schema.names

    def test_has_fmt_gq_column(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        assert "FMT_GQ" in info_fmt_batch.to_arrow().schema.names

    def test_info_dp_type(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        assert info_fmt_batch.to_arrow().schema.field("INFO_DP").type == pa.int32()

    def test_info_af_type(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        assert info_fmt_batch.to_arrow().schema.field("INFO_AF").type == pa.float32()

    def test_info_db_type(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        assert info_fmt_batch.to_arrow().schema.field("INFO_DB").type == pa.bool_()

    def test_info_dp_values(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_DP")
        assert col[0].as_py() == 20
        assert col[1].as_py() == 15

    def test_info_af_values(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_AF")
        assert abs(col[0].as_py() - 0.5) < 0.001
        assert abs(col[1].as_py() - 0.3) < 0.001

    def test_info_db_values(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_DB")
        assert col[0].as_py() is True
        assert col[1].as_py() is False

    def test_fmt_gt_both_samples(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GT")
        val = col[0].as_py()
        assert val is not None
        parts = val.split("\t")
        assert len(parts) == 2

    def test_fmt_gt_values(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GT")
        row0 = col[0].as_py().split("\t")
        assert row0[0] in ("0/1", "0|1")
        assert row0[1] in ("1/1", "1|1")

    def test_fmt_gq_values(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GQ")
        parts = col[0].as_py().split("\t")
        assert parts[0] == "40"
        assert parts[1] == "50"

    def test_missing_info_is_null(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_DB")
        assert col[1].as_py() is False  # DB absent on row 2 → false for Flag

    def test_to_arrow(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        b = info_fmt_batch.to_arrow()
        assert isinstance(b, pa.RecordBatch)
        assert b.num_rows == 2

    def test_to_polars(self, info_fmt_batch):
        pl = pytest.importorskip("polars")
        df = info_fmt_batch.to_polars()
        assert isinstance(df, pl.DataFrame)
        assert df.height == 2

    def test_to_iterator(self, info_fmt_batch):
        recs = list(info_fmt_batch.to_iterator())
        assert len(recs) == 2

    def test_from_arrow_roundtrip(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(info_fmt_batch.to_arrow())
        assert len(b2) == 2

    def test_to_iterator_raises_on_external(self, info_fmt_batch):
        pytest.importorskip("pyarrow")
        b2 = RecordBatch.from_arrow(info_fmt_batch.to_arrow())
        with pytest.raises(OSError):
            b2.to_iterator()
