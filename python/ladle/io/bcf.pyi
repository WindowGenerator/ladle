from __future__ import annotations
from types import TracebackType
from typing import Any

from ladle.io.core import Position, Region
from ladle.io.vcf import Header

class Record:
    """A single BCF variant record."""

    def reference_sequence_name(self, header: Header) -> str:
        """Chromosome / reference sequence name, resolved via the header string maps."""
        ...
    def variant_start(self) -> Position | None:
        """1-based variant start position, or ``None`` if unset."""
        ...
    def ids(self) -> list[str]:
        """Variant identifiers (ID field), e.g. rsIDs."""
        ...
    def reference_bases(self) -> bytes:
        """Reference allele bases (REF field) as bytes."""
        ...
    def alternate_bases(self) -> list[str]:
        """Alternate allele strings (ALT field)."""
        ...
    def quality_score(self) -> float | None:
        """Phred-scaled variant quality (QUAL field), or ``None`` if missing."""
        ...
    def filters(self, header: Header) -> list[str]:
        """Filter labels applied to this record (FILTER field)."""
        ...
    def info(
        self, header: Header
    ) -> dict[str, int | float | bool | str | list[int | float | str | None] | None]:
        """INFO field values as a ``{key: value}`` dict."""
        ...
    def __repr__(self) -> str: ...

class Reader:
    """Sequential BCF reader.

    Examples
    --------
    >>> with bcf.Reader.from_path("variants.bcf") as reader:
    ...     header = reader.read_header()
    ...     for record in reader:
    ...         print(record.variant_start())
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a BCF file at the given path."""
        ...
    def read_header(self) -> Header:
        """Parse and return the VCF/BCF header."""
        ...
    def records_to_batch(self, header: Header) -> RecordBatch:
        """Read all remaining records into a ``RecordBatch`` (Arrow-compatible)."""
        ...
    def __iter__(self) -> Reader: ...
    def __next__(self) -> Record: ...
    def close(self) -> None: ...
    def __enter__(self) -> Reader: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None: ...
    def __repr__(self) -> str: ...

class RecordBatch:
    """A batch of BCF records in columnar Arrow format.

    Examples
    --------
    >>> with bcf.Reader.from_path("variants.bcf") as reader:
    ...     header = reader.read_header()
    ...     batch = reader.records_to_batch(header)
    ... df = batch.to_polars()
    """

    @staticmethod
    def from_arrow(batch: Any) -> RecordBatch:
        """Create from a PyArrow ``RecordBatch``, ``Table``, or any object with ``__arrow_c_stream__``."""
        ...
    @staticmethod
    def from_polars(df: Any) -> RecordBatch:
        """Create from a Polars ``DataFrame``."""
        ...
    @staticmethod
    def from_pandas(df: Any) -> RecordBatch:
        """Create from a Pandas ``DataFrame``."""
        ...
    def to_arrow(self) -> Any:
        """Export as a PyArrow ``RecordBatch``."""
        ...
    def to_polars(self) -> Any:
        """Export as a Polars ``DataFrame``."""
        ...
    def to_pandas(self) -> Any:
        """Export as a Pandas ``DataFrame``."""
        ...
    def to_iterator(self) -> RecordBatchIterator:
        """Return an iterator that yields individual ``Record`` objects.

        Only available when the batch was created by ``Reader.records_to_batch()``,
        not from ``from_arrow`` / ``from_polars`` / ``from_pandas``.
        """
        ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class RecordBatchIterator:
    """Iterator that yields individual BCF ``Record`` objects from a ``RecordBatch``."""

    def __iter__(self) -> RecordBatchIterator: ...
    def __next__(self) -> Record: ...

class Query:
    """Iterator over BCF records in a queried genomic region."""

    def __iter__(self) -> Query: ...
    def __next__(self) -> Record: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class IndexedReader:
    """BCF reader with CSI index support for region-based queries.

    Examples
    --------
    >>> with bcf.IndexedReader.from_path("variants.bcf") as reader:
    ...     header = reader.read_header()
    ...     region = core.Region(b"chr1", core.Interval(1, 1000))
    ...     for record in reader.query(header, region):
    ...         print(record.variant_start())
    """

    @staticmethod
    def from_path(path: str) -> IndexedReader:
        """Open an indexed BCF file. Expects a ``.csi`` index alongside the BCF file."""
        ...
    def read_header(self) -> Header:
        """Parse and return the VCF/BCF header."""
        ...
    def query(self, header: Header, region: Region) -> Query:
        """Query records overlapping the given genomic region."""
        ...
    def close(self) -> None: ...
    def __enter__(self) -> IndexedReader: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None: ...
    def __repr__(self) -> str: ...

class Writer:
    """BCF writer (BGZF-compressed binary VCF).

    Examples
    --------
    >>> with bcf.Writer.from_path("out.bcf") as writer:
    ...     writer.write_header(header)
    ...     writer.write_record(header, record)
    """

    @staticmethod
    def from_path(path: str) -> Writer:
        """Create a new BCF file at the given path, overwriting if it exists."""
        ...
    def write_header(self, header: Header) -> None:
        """Write the VCF/BCF header. Must be called before ``write_record``."""
        ...
    def write_record(self, header: Header, record: Record) -> None:
        """Write a BCF record."""
        ...
    def close(self) -> None: ...
    def __enter__(self) -> Writer: ...
    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None: ...
    def __repr__(self) -> str: ...
