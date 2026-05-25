from __future__ import annotations
from types import TracebackType

COMPRESSION_NONE: int
COMPRESSION_FAST: int
COMPRESSION_BEST: int

class VirtualPosition:
    """A BGZF virtual position encoding both block offset and within-block offset.

    Virtual positions are used with ``Reader.seek`` for random access within a BGZF file.

    Examples
    --------
    >>> vpos = bgzf.VirtualPosition(block_offset, within_offset)
    >>> reader.seek(vpos)
    """

    MIN: VirtualPosition
    MAX: VirtualPosition

    def __new__(cls, compressed: int, uncompressed: int) -> VirtualPosition:
        """Create a virtual position from compressed block offset and within-block offset."""
        ...
    @staticmethod
    def from_u64(n: int) -> VirtualPosition:
        """Create a virtual position from its raw 64-bit integer encoding."""
        ...
    def compressed(self) -> int:
        """Compressed (block-level) offset in the BGZF file."""
        ...
    def uncompressed(self) -> int:
        """Within-block (uncompressed) byte offset."""
        ...
    def __int__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __lt__(self, other: VirtualPosition) -> bool: ...
    def __le__(self, other: VirtualPosition) -> bool: ...
    def __gt__(self, other: VirtualPosition) -> bool: ...
    def __ge__(self, other: VirtualPosition) -> bool: ...

class Reader:
    """BGZF (block gzip) reader with random-access support via virtual positions.

    Examples
    --------
    >>> with bgzf.Reader.from_path("data.bgz") as reader:
    ...     data = reader.read_all()
    """

    @staticmethod
    def from_path(path: str) -> Reader:
        """Open a BGZF file at the given path."""
        ...
    def read(self, size: int) -> bytes:
        """Read up to *size* decompressed bytes."""
        ...
    def read_all(self) -> bytes:
        """Read all remaining decompressed bytes."""
        ...
    def read_line(self) -> bytes:
        """Read one decompressed line (including the trailing ``\\n``)."""
        ...
    def position(self) -> int:
        """Current uncompressed stream position."""
        ...
    def virtual_position(self) -> VirtualPosition:
        """Current BGZF virtual position (block offset + within-block offset)."""
        ...
    def seek(self, vpos: VirtualPosition) -> VirtualPosition:
        """Seek to a virtual position (as returned by ``virtual_position()``)."""
        ...
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
    """BGZF (block gzip) writer.

    Examples
    --------
    >>> with bgzf.Writer.from_path("out.bgz") as writer:
    ...     writer.write(b"hello")
    ...     writer.finish()
    """

    @staticmethod
    def from_path(path: str, compression_level: int = 6) -> Writer:
        """Create a new BGZF file at the given path.

        Parameters
        ----------
        compression_level : int
            Zlib compression level (0 = none, 1 = fast, 9 = best). Default is 6.
        """
        ...
    def write(self, data: bytes) -> int:
        """Write compressed bytes. Returns the number of bytes written."""
        ...
    def flush(self) -> None:
        """Flush buffered data to the underlying file."""
        ...
    def finish(self) -> None:
        """Finalize the BGZF stream and write the EOF block."""
        ...
    def position(self) -> int:
        """Current uncompressed stream position."""
        ...
    def virtual_position(self) -> VirtualPosition:
        """Current BGZF virtual position."""
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

class gzi:
    class Index:
        """GZI (gzip index) for random access into a BGZF-compressed file.

        Examples
        --------
        >>> idx = bgzf.gzi.Index.from_path("data.bgz.gzi")
        >>> vpos = idx.query(1000)
        """

        @staticmethod
        def from_path(path: str) -> gzi.Index:
            """Read a ``.gzi`` index file from the given path."""
            ...
        def query(self, pos: int) -> VirtualPosition:
            """Return the virtual position for the BGZF block containing uncompressed offset *pos*."""
            ...
        def __len__(self) -> int: ...
        def __repr__(self) -> str: ...
