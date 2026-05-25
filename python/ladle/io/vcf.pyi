from __future__ import annotations
import io
from types import TracebackType
from typing import Any

from ladle.io.core import Position

class Header:
    """VCF header containing sample names, contig lengths, INFO/FORMAT field definitions.

    Examples
    --------
    >>> with vcf.Reader.from_path("variants.vcf") as reader:
    ...     header = reader.read_header()
    ...     print(header.sample_names())
    ...     print(header.contigs())
    """

    def __new__(cls) -> Header: ...
    @staticmethod
    def parse(s: str) -> Header:
        """Parse a VCF header from its string representation."""
        ...
    def sample_names(self) -> list[str]:
        """Return sample names in column order."""
        ...
    def contigs(self) -> dict[str, int | None]:
        """Return contig metadata as ``{name: length_or_None}`` dict."""
        ...
    def info_fields(self) -> list[tuple[str, str]]:
        """Return INFO field definitions as ``[(key, type_str)]`` list."""
        ...
    def format_fields(self) -> list[tuple[str, str]]:
        """Return FORMAT field definitions as ``[(key, type_str)]`` list."""
        ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Record:
    """A single VCF variant record."""

    def reference_sequence_name(self) -> str:
        """Chromosome / reference sequence name (CHROM field)."""
        ...
    def variant_start(self) -> Position | None:
        """1-based variant start position (POS field), or ``None`` if unset."""
        ...
    def ids(self) -> list[str]:
        """Variant identifiers (ID field), e.g. rsIDs."""
        ...
    def reference_bases(self) -> str:
        """Reference allele bases (REF field)."""
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
    """Sequential VCF reader.

    Examples
    --------
    >>> with vcf.Reader.from_path("variants.vcf") as reader:
    ...     header = reader.read_header()
    ...     for record in reader:
    ...         print(record.reference_sequence_name())
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a VCF file at the given path."""
        ...
    @staticmethod
    def from_fd(fd: io.RawIOBase) -> Reader:
        """Open a VCF reader from an open file descriptor (Unix only)."""
        ...
    def read_header(self) -> Header:
        """Parse and return the VCF header."""
        ...
    def records_to_batch(self, header: Header | None = None) -> RecordBatch:
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
    """A batch of VCF records in columnar Arrow format.

    Examples
    --------
    >>> with vcf.Reader.from_path("variants.vcf") as reader:
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
    """Iterator that yields individual VCF ``Record`` objects from a ``RecordBatch``."""

    def __iter__(self) -> RecordBatchIterator: ...
    def __next__(self) -> Record: ...

class Writer:
    """VCF writer.

    Examples
    --------
    >>> with vcf.Writer.from_path("out.vcf") as writer:
    ...     writer.write_header(header)
    ...     writer.write_record(header, record)
    """

    @staticmethod
    def from_path(path: str) -> Writer:
        """Create a new VCF file at the given path, overwriting if it exists."""
        ...
    def write_header(self, header: Header) -> None:
        """Write the VCF header. Must be called before ``write_record``."""
        ...
    def write_record(self, header: Header, record: Record) -> None:
        """Write a VCF record."""
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
