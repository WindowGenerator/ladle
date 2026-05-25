from __future__ import annotations
import io
from types import TracebackType
from typing import Any, TypeVar

from ladle.io.core import Position, Region
from ladle.io.sam import Flags, Header, MappingQuality
import ladle.io.sam as sam

Num = TypeVar("Num", bound=float | int)

class Record:
    """A single BAM alignment record."""

    def name(self) -> bytes | None:
        """Read name (QNAME), or ``None`` if absent."""
        ...
    def flags(self) -> Flags:
        """SAM flags as a ``Flags`` bitset."""
        ...
    def reference_sequence_id(self) -> int | None:
        """0-based index into the header reference sequence dictionary, or ``None`` if unmapped."""
        ...
    def alignment_start(self) -> Position | None:
        """1-based alignment start position, or ``None`` if unmapped."""
        ...
    def mapping_quality(self) -> MappingQuality | None:
        """Mapping quality (MAPQ), or ``None`` if unavailable (value 255)."""
        ...
    def cigar(self) -> bytes:
        """Raw CIGAR bytes."""
        ...
    def mate_reference_sequence_id(self) -> int | None:
        """Reference sequence index of the mate, or ``None``."""
        ...
    def mate_alignment_start(self) -> Position | None:
        """1-based alignment start of the mate, or ``None``."""
        ...
    def template_length(self) -> int:
        """Observed template length (TLEN). Negative for reverse-strand mates."""
        ...
    def sequence(self) -> bytes:
        """Nucleotide sequence (SEQ) as bytes."""
        ...
    def quality_scores(self) -> bytes:
        """Base quality scores (QUAL) as raw Phred bytes."""
        ...
    def data(self) -> dict[bytes, int | float | bytes | list[Num]]:
        """Optional auxiliary fields (TAG:TYPE:VALUE) as a ``{bytes: value}`` dict."""
        ...
    def __repr__(self) -> str: ...

class Reader:
    """Sequential BAM reader.

    Iterates over records in file order. Use ``IndexedReader`` for random access by region.

    Examples
    --------
    >>> with bam.Reader.from_path("reads.bam") as reader:
    ...     header = reader.read_header()
    ...     for record in reader:
    ...         print(record.name())
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a BAM file at the given path."""
        ...
    @staticmethod
    def from_fd(fd: io.RawIOBase) -> Reader:
        """Open a BAM reader from an open file descriptor (Unix only)."""
        ...
    def read_header(self) -> Header:
        """Parse and return the SAM/BAM header."""
        ...
    def records_to_batch(self) -> RecordBatch:
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

class Query:
    """Iterator over BAM records in a queried genomic region."""

    def __iter__(self) -> Query: ...
    def __next__(self) -> Record: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class IndexedReader:
    """BAM reader with ``.bai`` index support for region-based queries.

    Examples
    --------
    >>> with bam.IndexedReader.from_path("reads.bam") as reader:
    ...     header = reader.read_header()
    ...     region = core.Region(b"chr1", core.Interval(1, 1000))
    ...     for record in reader.query(header, region):
    ...         print(record.name())
    """

    @staticmethod
    def from_path(path: str) -> IndexedReader:
        """Open an indexed BAM file. Expects a ``.bai`` index alongside the BAM file."""
        ...
    def read_header(self) -> Header:
        """Parse and return the SAM/BAM header."""
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
    """BAM writer. Must call ``write_header`` before writing any records.

    Examples
    --------
    >>> with bam.Writer.from_path("out.bam") as writer:
    ...     writer.write_header(header)
    ...     writer.write_record(header, record)
    """

    @staticmethod
    def from_path(path: str) -> Writer:
        """Create a new BAM file at the given path, overwriting if it exists."""
        ...
    def write_header(self, header: Header) -> None:
        """Write the SAM/BAM header. Must be called before ``write_record``."""
        ...
    def write_record(self, header: Header, record: Record) -> None:
        """Write a BAM record."""
        ...
    def write_sam_record(self, header: Header, record: sam.Record) -> None:
        """Write a SAM record into this BAM file (cross-format conversion)."""
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

class RecordBatch:
    """A batch of BAM records in columnar Arrow format.

    Columns: ``name``, ``flags``, ``reference_sequence_id``, ``alignment_start``,
    ``mapping_quality``, ``cigar``, ``mate_reference_sequence_id``,
    ``mate_alignment_start``, ``template_length``, ``sequence``, ``quality_scores``.

    Examples
    --------
    >>> with bam.Reader.from_path("reads.bam") as reader:
    ...     reader.read_header()
    ...     batch = reader.records_to_batch()
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
    """Iterator that yields individual BAM ``Record`` objects from a ``RecordBatch``."""

    def __iter__(self) -> RecordBatchIterator: ...
    def __next__(self) -> Record: ...

class bai:
    class Index:
        """BAI (BAM index) loaded from a ``.bai`` file."""

        @staticmethod
        def read_from_path(path: str) -> bai.Index:
            """Read a ``.bai`` index file from the given path."""
            ...
        def __repr__(self) -> str: ...
