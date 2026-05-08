import pytest
from ladle.io.core import Interval, Position, Region


class TestPosition:
    def test_basic(self):
        assert Position(1).get() == 1
        assert Position(100).get() == 100

    def test_zero_raises(self):
        with pytest.raises(ValueError):
            Position(0)

    def test_int(self):
        assert int(Position(5)) == 5

    def test_str(self):
        assert str(Position(42)) == "42"

    def test_repr(self):
        assert repr(Position(42)) == "Position(42)"

    def test_equality(self):
        assert Position(5) == Position(5)
        assert Position(5) != Position(6)

    def test_ordering(self):
        assert Position(5) < Position(10)
        assert Position(10) > Position(5)
        assert Position(5) <= Position(5)
        assert Position(5) >= Position(5)

    def test_hash(self):
        assert hash(Position(5)) == hash(Position(5))
        assert hash(Position(5)) != hash(Position(6))

    def test_min_max(self):
        assert Position.MIN.get() == 1
        assert Position.MAX.get() > 0
        assert Position.MIN < Position.MAX

    def test_parse(self):
        assert Position.parse("42").get() == 42
        assert Position.parse("1").get() == 1

    def test_parse_zero_raises(self):
        with pytest.raises(ValueError):
            Position.parse("0")

    def test_parse_invalid_raises(self):
        with pytest.raises(ValueError):
            Position.parse("abc")

    def test_checked_add(self):
        assert Position(5).checked_add(3).get() == 8
        assert Position(5).checked_add(0).get() == 5

    def test_checked_add_overflow(self):
        assert Position.MAX.checked_add(1) is None


class TestInterval:
    def test_both_bounds(self):
        i = Interval(1, 100)
        assert i.start().get() == 1
        assert i.end().get() == 100

    def test_start_only(self):
        i = Interval(8, None)
        assert i.start().get() == 8
        assert i.end() is None

    def test_end_only(self):
        i = Interval(None, 100)
        assert i.start() is None
        assert i.end().get() == 100

    def test_unbounded(self):
        i = Interval()
        assert i.start() is None
        assert i.end() is None

    def test_contains(self):
        i = Interval(1, 100)
        assert i.contains(Position(50)) is True
        assert i.contains(Position(1)) is True
        assert i.contains(Position(100)) is True
        assert i.contains(Position(101)) is False

    def test_intersects(self):
        a = Interval(1, 100)
        b = Interval(50, 150)
        c = Interval(200, 300)
        assert a.intersects(b) is True
        assert a.intersects(c) is False

    def test_str(self):
        assert str(Interval(8, 13)) == "8-13"
        assert str(Interval(8, None)) == "8"
        assert str(Interval(None, 13)) == "1-13"
        assert str(Interval()) == ""

    def test_equality(self):
        assert Interval(1, 100) == Interval(1, 100)
        assert Interval(1, 100) != Interval(1, 101)

    def test_parse(self):
        assert Interval.parse("8-13") == Interval(8, 13)
        assert Interval.parse("8") == Interval(8, None)
        assert Interval.parse("") == Interval()

    def test_zero_raises(self):
        with pytest.raises(ValueError):
            Interval(0, 100)


class TestRegion:
    def test_str_name(self):
        r = Region("chr1", Interval(100, 200))
        assert r.name() == b"chr1"
        assert r.start().get() == 100
        assert r.end().get() == 200

    def test_bytes_name(self):
        r = Region(b"chr1", Interval(100, 200))
        assert r.name() == b"chr1"

    def test_str(self):
        r = Region("chr1", Interval(100, 200))
        assert str(r) == "chr1:100-200"

    def test_unbounded_interval(self):
        r = Region("chr1", Interval())
        assert r.start() is None
        assert r.end() is None
        assert str(r) == "chr1"

    def test_interval(self):
        i = Interval(100, 200)
        r = Region("chr1", i)
        assert r.interval() == i

    def test_equality(self):
        assert Region("chr1", Interval(1, 100)) == Region("chr1", Interval(1, 100))
        assert Region("chr1", Interval(1, 100)) != Region("chr2", Interval(1, 100))

    def test_parse(self):
        r = Region.parse("chr1:100-200")
        assert r.name() == b"chr1"
        assert r.start().get() == 100
        assert r.end().get() == 200

    def test_parse_no_interval(self):
        r = Region.parse("chr1")
        assert r.name() == b"chr1"
        assert r.start() is None
        assert r.end() is None

    def test_parse_invalid_raises(self):
        with pytest.raises(ValueError):
            Region.parse("")
