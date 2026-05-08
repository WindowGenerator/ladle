import pytest
from ladle.io.bgzf import (
    COMPRESSION_BEST,
    COMPRESSION_FAST,
    COMPRESSION_NONE,
    Reader,
    VirtualPosition,
    Writer,
)


class TestVirtualPosition:
    def test_basic(self):
        vp = VirtualPosition(0, 0)
        assert vp.compressed() == 0
        assert vp.uncompressed() == 0

    def test_values(self):
        vp = VirtualPosition(100, 5)
        assert vp.compressed() == 100
        assert vp.uncompressed() == 5

    def test_int(self):
        vp = VirtualPosition(100, 5)
        assert int(vp) == (100 << 16) | 5

    def test_from_u64(self):
        raw = (100 << 16) | 5
        vp = VirtualPosition.from_u64(raw)
        assert vp.compressed() == 100
        assert vp.uncompressed() == 5

    def test_from_u64_roundtrip(self):
        vp = VirtualPosition(42, 7)
        assert VirtualPosition.from_u64(int(vp)) == vp

    def test_overflow_raises(self):
        with pytest.raises(ValueError):
            VirtualPosition(2**48, 0)

    def test_min_max(self):
        assert VirtualPosition.MIN <= VirtualPosition.MAX
        assert VirtualPosition.MIN.compressed() == 0
        assert VirtualPosition.MIN.uncompressed() == 0

    def test_equality(self):
        assert VirtualPosition(10, 3) == VirtualPosition(10, 3)
        assert VirtualPosition(10, 3) != VirtualPosition(10, 4)

    def test_ordering(self):
        a = VirtualPosition(10, 0)
        b = VirtualPosition(20, 0)
        c = VirtualPosition(10, 5)
        assert a < b
        assert b > a
        assert a < c
        assert a <= a
        assert a >= a

    def test_hash(self):
        assert hash(VirtualPosition(10, 3)) == hash(VirtualPosition(10, 3))

    def test_repr(self):
        vp = VirtualPosition(10, 3)
        assert "10" in repr(vp)
        assert "3" in repr(vp)

    def test_str(self):
        vp = VirtualPosition(10, 3)
        assert str(vp) == "10:3"


class TestCompression:
    def test_constants_exist(self):
        assert isinstance(COMPRESSION_NONE, int)
        assert isinstance(COMPRESSION_FAST, int)
        assert isinstance(COMPRESSION_BEST, int)

    def test_none_is_zero(self):
        assert COMPRESSION_NONE == 0

    def test_fast_is_one(self):
        assert COMPRESSION_FAST == 1

    def test_ordering(self):
        assert COMPRESSION_NONE < COMPRESSION_FAST <= COMPRESSION_BEST


class TestRoundTrip:
    def test_write_read(self, tmp_path):
        path = str(tmp_path / "test.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"hello bgzf")
        with Reader.from_path(path) as r:
            data = r.read(100)
        assert data == b"hello bgzf"

    def test_write_read_multiblock(self, tmp_path):
        path = str(tmp_path / "multi.bgzf")
        payload = b"A" * 65536 + b"B" * 1000
        with Writer.from_path(path) as w:
            w.write(payload)
        with Reader.from_path(path) as r:
            assert r.read_all() == payload

    def test_compression_levels(self, tmp_path):
        payload = b"x" * 10000
        for level in (COMPRESSION_NONE, COMPRESSION_FAST, COMPRESSION_BEST):
            path = str(tmp_path / f"level_{level}.bgzf")
            with Writer.from_path(path, compression_level=level) as w:
                w.write(payload)
            with Reader.from_path(path) as r:
                assert r.read_all() == payload

    def test_empty_file(self, tmp_path):
        path = str(tmp_path / "empty.bgzf")
        with Writer.from_path(path):
            pass
        with Reader.from_path(path) as r:
            assert r.read(10) == b""

    def test_read_returns_empty_on_eof(self, tmp_path):
        path = str(tmp_path / "eof.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"hi")
        with Reader.from_path(path) as r:
            r.read(100)
            assert r.read(10) == b""


class TestVirtualPositionIO:
    def test_virtual_position_advances(self, tmp_path):
        path = str(tmp_path / "vp.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"hello bgzf world")
        with Reader.from_path(path) as r:
            vp0 = r.virtual_position()
            r.read(5)
            vp1 = r.virtual_position()
            assert vp1 >= vp0

    def test_seek_roundtrip(self, tmp_path):
        path = str(tmp_path / "seek.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"hello bgzf")
        with Reader.from_path(path) as r:
            r.read(5)
            vp = r.virtual_position()
            r.seek(VirtualPosition(0, 0))
            assert r.read(5) == b"hello"
            r.seek(vp)
            assert r.read(5) == b" bgzf"

    def test_position(self, tmp_path):
        path = str(tmp_path / "pos.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"data")
        with Reader.from_path(path) as r:
            assert isinstance(r.position(), int)


class TestReaderWriter:
    def test_context_manager_closes(self, tmp_path):
        path = str(tmp_path / "cm.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"test")
        with Reader.from_path(path) as r:
            pass
        assert "closed" in repr(r)

    def test_read_after_close_raises(self, tmp_path):
        path = str(tmp_path / "closed.bgzf")
        with Writer.from_path(path) as w:
            w.write(b"x")
        r = Reader.from_path(path)
        r.close()
        with pytest.raises(OSError):
            r.read(10)

    def test_write_after_close_raises(self, tmp_path):
        path = str(tmp_path / "wclosed.bgzf")
        w = Writer.from_path(path)
        w.finish()
        with pytest.raises(OSError):
            w.write(b"x")

    def test_explicit_finish(self, tmp_path):
        path = str(tmp_path / "finish.bgzf")
        w = Writer.from_path(path)
        w.write(b"explicit")
        w.finish()
        with Reader.from_path(path) as r:
            assert r.read(100) == b"explicit"

    def test_invalid_compression_raises(self, tmp_path):
        with pytest.raises((ValueError, OSError)):
            Writer.from_path(str(tmp_path / "bad.bgzf"), compression_level=200)

    def test_file_not_found_raises(self):
        with pytest.raises(OSError):
            Reader.from_path("/nonexistent/path/file.bgzf")
