from __future__ import annotations
from types import TracebackType
from typing import Any

from ladle.io.core import Position

class Flags:
    """SAM/BAM alignment flags bitset.

    Class-level constants (e.g. ``Flags.UNMAPPED``) hold the individual bit values.
    Use bitwise operators (``&``, ``|``) to combine or test flags.

    Examples
    --------
    >>> flags = sam.Flags(0x0003)
    >>> flags.is_segmented()
    True
    >>> bool(flags & sam.Flags(sam.Flags.UNMAPPED))
    False
    """

    SEGMENTED: int
    PROPERLY_SEGMENTED: int
    UNMAPPED: int
    MATE_UNMAPPED: int
    REVERSE_COMPLEMENTED: int
    MATE_REVERSE_COMPLEMENTED: int
    FIRST_SEGMENT: int
    LAST_SEGMENT: int
    SECONDARY: int
    QC_FAIL: int
    DUPLICATE: int
    SUPPLEMENTARY: int

    def __new__(cls, bits: int) -> Flags: ...
    def bits(self) -> int:
        """Return the raw 16-bit integer value of the flags."""
        ...
    def is_segmented(self) -> bool:
        """Read is part of a template with multiple segments (0x0001)."""
        ...
    def is_properly_segmented(self) -> bool:
        """All segments are properly aligned (0x0002)."""
        ...
    def is_unmapped(self) -> bool:
        """Segment is unmapped (0x0004)."""
        ...
    def is_mate_unmapped(self) -> bool:
        """Next segment in template is unmapped (0x0008)."""
        ...
    def is_reverse_complemented(self) -> bool:
        """Sequence is reverse complemented (0x0010)."""
        ...
    def is_mate_reverse_complemented(self) -> bool:
        """Next segment sequence is reverse complemented (0x0020)."""
        ...
    def is_first_segment(self) -> bool:
        """This is the first segment in the template (0x0040)."""
        ...
    def is_last_segment(self) -> bool:
        """This is the last segment in the template (0x0080)."""
        ...
    def is_secondary(self) -> bool:
        """Secondary alignment (0x0100)."""
        ...
    def is_qc_fail(self) -> bool:
        """Read fails quality controls (0x0200)."""
        ...
    def is_duplicate(self) -> bool:
        """PCR or optical duplicate (0x0400)."""
        ...
    def is_supplementary(self) -> bool:
        """Supplementary alignment (0x0800)."""
        ...
    def __int__(self) -> int: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __and__(self, other: Flags) -> Flags: ...
    def __or__(self, other: Flags) -> Flags: ...

class MappingQuality:
    """Mapping quality score (MAPQ) in the range 0–254. Value 255 means unavailable.

    Examples
    --------
    >>> mq = sam.MappingQuality(60)
    >>> mq.get()
    60
    """

    MIN: MappingQuality
    MAX: MappingQuality

    def __new__(cls, n: int) -> MappingQuality: ...
    def get(self) -> int:
        """Return the raw MAPQ byte value (0–254)."""
        ...
    def __int__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...
    def __lt__(self, other: MappingQuality) -> bool: ...
    def __le__(self, other: MappingQuality) -> bool: ...
    def __gt__(self, other: MappingQuality) -> bool: ...
    def __ge__(self, other: MappingQuality) -> bool: ...

class Header:
    """SAM/BAM header containing reference sequences and other metadata.

    Examples
    --------
    >>> with sam.Reader.from_path("reads.sam") as reader:
    ...     header = reader.read_header()
    ...     print(header.reference_sequences())
    """

    def __new__(cls) -> Header: ...
    @staticmethod
    def parse(s: str) -> Header:
        """Parse a SAM header from its string representation."""
        ...
    def reference_sequences(self) -> dict[bytes, int]:
        """Return reference sequences as ``{name_bytes: length}`` dict."""
        ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Record:
    """A single SAM alignment record."""

    def name(self) -> bytes | None:
        """Read name (QNAME), or ``None`` if absent."""
        ...
    def flags(self) -> Flags:
        """SAM flags as a ``Flags`` bitset."""
        ...
    def reference_sequence_name(self) -> bytes | None:
        """Reference sequence name, or ``None`` if unmapped."""
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
    def mate_reference_sequence_name(self) -> bytes | None:
        """Reference sequence name of the mate, or ``None``."""
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
    def data(self) -> dict[bytes, int | float | bytes | list[int] | list[float]]:
        """Optional auxiliary fields (TAG:TYPE:VALUE) as a ``{bytes: value}`` dict."""
        ...
    def __repr__(self) -> str: ...

class Reader:
    """Sequential SAM reader.

    Examples
    --------
    >>> with sam.Reader.from_path("reads.sam") as reader:
    ...     header = reader.read_header()
    ...     for record in reader:
    ...         print(record.name())
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a SAM file at the given path."""
        ...
    def read_header(self) -> Header:
        """Parse and return the SAM header."""
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

class RecordBatch:
    """A batch of SAM records in columnar Arrow format.

    Columns: ``name``, ``flags``, ``reference_sequence_name``, ``alignment_start``,
    ``mapping_quality``, ``cigar``, ``mate_reference_sequence_name``,
    ``mate_alignment_start``, ``template_length``, ``sequence``, ``quality_scores``.

    Examples
    --------
    >>> with sam.Reader.from_path("reads.sam") as reader:
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
    """Iterator that yields individual SAM ``Record`` objects from a ``RecordBatch``."""

    def __iter__(self) -> RecordBatchIterator: ...
    def __next__(self) -> Record: ...

class Writer:
    """SAM writer.

    Examples
    --------
    >>> with sam.Writer.from_path("out.sam") as writer:
    ...     writer.write_header(header)
    ...     writer.write_record(header, record)
    """

    @staticmethod
    def from_path(path: str) -> Writer:
        """Create a new SAM file at the given path, overwriting if it exists."""
        ...
    def write_header(self, header: Header) -> None:
        """Write the SAM header. Must be called before ``write_record``."""
        ...
    def write_record(self, header: Header, record: Record) -> None:
        """Write a SAM record."""
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
