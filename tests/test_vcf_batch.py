import os
import tempfile

import pytest

import ladle.io.vcf as vcf
from ladle.io.vcf import Reader, RecordBatch, RecordBatchIterator, Record

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

HEADER_TEXT = (
    "##fileformat=VCFv4.2\n"
    "##contig=<ID=chr1,length=248956422>\n"
    "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total depth\">\n"
    "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
)

RECORDS_TEXT = "".join(
    f"chr1\t{100 + i}\trs{i}\tA\tG\t{30.0 + i}\tPASS\tDP={10 + i}\n"
    for i in range(5)
)

VCF_TEXT = HEADER_TEXT + RECORDS_TEXT

MULTI_ALT_TEXT = HEADER_TEXT + "chr1\t200\trs99\tA\tG,T\t50.0\tPASS\tDP=20\n"

MISSING_QUAL_TEXT = HEADER_TEXT + "chr1\t300\t.\tA\tG\t.\tPASS\tDP=5\n"

NO_ALT_TEXT = HEADER_TEXT + "chr1\t400\t.\tA\t.\t.\t.\t.\n"


def _write_vcf(vcf_text: str, path: str) -> None:
    with open(path, "w") as f:
        f.write(vcf_text)


@pytest.fixture
def vcf_path(tmp_path):
    path = str(tmp_path / "test.vcf")
    _write_vcf(VCF_TEXT, path)
    return path


@pytest.fixture
def batch(vcf_path):
    with Reader.from_path(vcf_path) as r:
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

    def test_recordbatch_accessible_from_io_vcf(self):
        import ladle.io.vcf as io_vcf
        assert io_vcf.RecordBatch is RecordBatch


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
        path = str(tmp_path / "empty.vcf")
        _write_vcf(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert len(b) == 0


# ---------------------------------------------------------------------------
# to_arrow
# ---------------------------------------------------------------------------

class TestToArrow:
    def test_returns_record_batch(self, batch):
        pa = pytest.importorskip("pyarrow")
        b = batch.to_arrow()
        assert isinstance(b, pa.RecordBatch)

    def test_num_rows(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().num_rows == 5

    def test_schema_field_names(self, batch):
        pa = pytest.importorskip("pyarrow")
        names = batch.to_arrow().schema.names
        for field in ("chrom", "pos", "id", "ref", "alt", "qual"):
            assert field in names

    def test_chrom_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("chrom").type == pa.large_utf8()

    def test_pos_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("pos").type == pa.int32()

    def test_qual_type(self, batch):
        pa = pytest.importorskip("pyarrow")
        assert batch.to_arrow().schema.field("qual").type == pa.float32()

    def test_chrom_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("chrom")
        assert all(col[i].as_py() == "chr1" for i in range(5))

    def test_pos_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("pos")
        assert col[0].as_py() == 100
        assert col[4].as_py() == 104

    def test_id_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("id")
        assert col[0].as_py() == "rs0"

    def test_ref_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("ref")
        assert col[0].as_py() == "A"

    def test_alt_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("alt")
        assert col[0].as_py() == "G"

    def test_qual_values(self, batch):
        pa = pytest.importorskip("pyarrow")
        col = batch.to_arrow().column("qual")
        assert abs(col[0].as_py() - 30.0) < 0.01

    def test_missing_qual_is_null(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        path = str(tmp_path / "noqual.vcf")
        _write_vcf(MISSING_QUAL_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        col = b.to_arrow().column("qual")
        assert col[0].as_py() is None

    def test_no_alt_is_null(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        path = str(tmp_path / "noalt.vcf")
        _write_vcf(NO_ALT_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        col = b.to_arrow().column("alt")
        assert col[0].as_py() is None

    def test_multi_alt_joined(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        path = str(tmp_path / "multialt.vcf")
        _write_vcf(MULTI_ALT_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        col = b.to_arrow().column("alt")
        assert col[0].as_py() == "G,T"

    def test_missing_id_is_null(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        path = str(tmp_path / "noid.vcf")
        _write_vcf(HEADER_TEXT + "chr1\t100\t.\tA\tG\t.\t.\t.\n", path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        col = b.to_arrow().column("id")
        assert col[0].as_py() is None

    def test_empty_batch_arrow(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        path = str(tmp_path / "empty.vcf")
        _write_vcf(HEADER_TEXT, path)
        with Reader.from_path(path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert b.to_arrow().num_rows == 0


# ---------------------------------------------------------------------------
# to_iterator
# ---------------------------------------------------------------------------

class TestToIterator:
    def test_yields_records(self, batch):
        records = list(batch.to_iterator())
        assert len(records) == 5

    def test_yields_vcf_record_type(self, batch):
        for rec in batch.to_iterator():
            assert isinstance(rec, Record)

    def test_record_fields_accessible(self, batch):
        rec = next(iter(batch.to_iterator()))
        assert rec.reference_sequence_name() == "chr1"
        assert rec.reference_bases() == "A"

    def test_iterator_protocol(self, batch):
        it = batch.to_iterator()
        assert iter(it) is it
        first = next(it)
        assert isinstance(first, Record)

    def test_empty_batch_iterator(self, tmp_path):
        path = str(tmp_path / "empty.vcf")
        _write_vcf(HEADER_TEXT, path)
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
        pl = pytest.importorskip("polars")
        assert batch.to_polars().height == 5

    def test_column_names(self, batch):
        pl = pytest.importorskip("polars")
        cols = batch.to_polars().columns
        for c in ("chrom", "pos", "ref", "alt", "qual"):
            assert c in cols


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
        col = b2.to_arrow().column("chrom")
        assert all(col[i].as_py() == "chr1" for i in range(5))

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
        path = str(tmp_path / "empty.vcf")
        _write_vcf(HEADER_TEXT, path)
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
        col = b2.to_arrow().column("chrom")
        assert all(col[i].as_py() == "chr1" for i in range(5))

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


# ---------------------------------------------------------------------------
# records_to_batch(header) — INFO and FORMAT columns
# ---------------------------------------------------------------------------

INFO_FMT_HEADER = (
    "##fileformat=VCFv4.2\n"
    "##contig=<ID=chr1,length=248956422>\n"
    "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"Total depth\">\n"
    "##INFO=<ID=AF,Number=A,Type=Float,Description=\"Allele freq\">\n"
    "##INFO=<ID=DB,Number=0,Type=Flag,Description=\"dbSNP\">\n"
    "##INFO=<ID=GENE,Number=1,Type=String,Description=\"Gene name\">\n"
    "##FORMAT=<ID=GT,Number=1,Type=String,Description=\"Genotype\">\n"
    "##FORMAT=<ID=GQ,Number=1,Type=Integer,Description=\"Genotype quality\">\n"
    "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tSAMPLE1\tSAMPLE2\n"
)

INFO_FMT_RECORDS = (
    "chr1\t100\trs1\tA\tG\t30.0\tPASS\tDP=20;AF=0.5;DB;GENE=TP53\tGT:GQ\t0/1:40\t1/1:50\n"
    "chr1\t200\trs2\tC\tT\t25.0\tPASS\tDP=15;AF=0.3;GENE=BRCA1\tGT:GQ\t0/0:60\t0/1:35\n"
)

INFO_FMT_VCF = INFO_FMT_HEADER + INFO_FMT_RECORDS


@pytest.fixture
def info_fmt_path(tmp_path):
    path = str(tmp_path / "info_fmt.vcf")
    _write_vcf(INFO_FMT_VCF, path)
    return path


@pytest.fixture
def info_fmt_batch(info_fmt_path):
    with Reader.from_path(info_fmt_path) as r:
        hdr = r.read_header()
        return r.records_to_batch(hdr)


class TestRecordsBatchWithHeader:
    def test_type(self, info_fmt_batch):
        assert isinstance(info_fmt_batch, RecordBatch)

    def test_len(self, info_fmt_batch):
        assert len(info_fmt_batch) == 2

    def test_has_info_dp_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "INFO_DP" in names

    def test_has_info_af_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "INFO_AF" in names

    def test_has_info_db_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "INFO_DB" in names

    def test_has_info_gene_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "INFO_GENE" in names

    def test_has_fmt_gt_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "FMT_GT" in names

    def test_has_fmt_gq_column(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        assert "FMT_GQ" in names

    def test_info_dp_type_is_int32(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        schema = info_fmt_batch.to_arrow().schema
        assert schema.field("INFO_DP").type == pa.int32()

    def test_info_af_type_is_float32(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        schema = info_fmt_batch.to_arrow().schema
        assert schema.field("INFO_AF").type == pa.float32()

    def test_info_db_type_is_bool(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        schema = info_fmt_batch.to_arrow().schema
        assert schema.field("INFO_DB").type == pa.bool_()

    def test_info_gene_type_is_string(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        schema = info_fmt_batch.to_arrow().schema
        assert schema.field("INFO_GENE").type == pa.large_utf8()

    def test_info_dp_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_DP")
        assert col[0].as_py() == 20
        assert col[1].as_py() == 15

    def test_info_af_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_AF")
        assert abs(col[0].as_py() - 0.5) < 0.001
        assert abs(col[1].as_py() - 0.3) < 0.001

    def test_info_db_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_DB")
        assert col[0].as_py() is True
        assert col[1].as_py() is False

    def test_info_gene_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("INFO_GENE")
        assert col[0].as_py() == "TP53"
        assert col[1].as_py() == "BRCA1"

    def test_fmt_gt_contains_both_samples(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GT")
        val = col[0].as_py()
        assert val is not None
        parts = val.split("\t")
        assert len(parts) == 2

    def test_fmt_gt_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GT")
        row0 = col[0].as_py().split("\t")
        assert "0/1" in row0[0] or row0[0] in ("0/1", "0|1")
        assert "1/1" in row0[1] or row0[1] in ("1/1", "1|1")

    def test_fmt_gq_values(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        col = info_fmt_batch.to_arrow().column("FMT_GQ")
        val = col[0].as_py()
        assert val is not None
        parts = val.split("\t")
        assert parts[0] == "40"
        assert parts[1] == "50"

    def test_base_columns_still_present(self, info_fmt_batch):
        pa = pytest.importorskip("pyarrow")
        names = info_fmt_batch.to_arrow().schema.names
        for col in ("chrom", "pos", "ref", "alt", "qual"):
            assert col in names

    def test_no_header_gives_6_columns(self, info_fmt_path):
        pa = pytest.importorskip("pyarrow")
        with Reader.from_path(info_fmt_path) as r:
            r.read_header()
            b = r.records_to_batch()
        assert b.to_arrow().num_columns == 6

    def test_missing_info_field_is_null(self, tmp_path):
        pa = pytest.importorskip("pyarrow")
        vcf_text = (
            "##fileformat=VCFv4.2\n"
            "##INFO=<ID=DP,Number=1,Type=Integer,Description=\"depth\">\n"
            "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\n"
            "chr1\t100\t.\tA\tG\t.\t.\t.\n"
        )
        path = str(tmp_path / "missing_info.vcf")
        _write_vcf(vcf_text, path)
        with Reader.from_path(path) as r:
            hdr = r.read_header()
            b = r.records_to_batch(hdr)
        col = b.to_arrow().column("INFO_DP")
        assert col[0].as_py() is None

    def test_repr_shows_column_count(self, info_fmt_batch):
        r = repr(info_fmt_batch)
        assert "RecordBatch" in r

    def test_to_iterator_works(self, info_fmt_batch):
        recs = list(info_fmt_batch.to_iterator())
        assert len(recs) == 2
