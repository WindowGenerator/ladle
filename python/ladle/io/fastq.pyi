from __future__ import annotations
from types import TracebackType

class Record:
    """A single FASTQ record with name, sequence, and quality scores."""

    def name(self) -> bytes:
        """Read name (without the leading ``@``)."""
        ...
    def description(self) -> bytes | None:
        """Optional description after the name on the header line, or ``None`` if absent."""
        ...
    def sequence(self) -> bytes:
        """Nucleotide sequence as bytes."""
        ...
    def quality_scores(self) -> bytes:
        """Base quality scores as raw Phred bytes."""
        ...
    def __repr__(self) -> str: ...

class Reader:
    """Sequential FASTQ reader.

    Examples
    --------
    >>> with fastq.Reader.from_path("reads.fastq") as reader:
    ...     for record in reader:
    ...         print(record.name(), record.sequence())
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a FASTQ file at the given path."""
        ...
    @staticmethod
    def from_fd(file: object) -> Reader:
        """Open a FASTQ reader from an open file descriptor (Unix only)."""
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

class Writer:
    """FASTQ writer.

    Examples
    --------
    >>> with fastq.Writer.from_path("out.fastq") as writer:
    ...     writer.write_record(record)
    """

    @staticmethod
    def from_path(path: str) -> Writer:
        """Create a new FASTQ file at the given path, overwriting if it exists."""
        ...
    def write_record(self, record: Record) -> None:
        """Write a FASTQ record."""
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
