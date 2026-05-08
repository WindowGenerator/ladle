# PyO3 `#[pymodule(submodule)]` registers submodules as attributes of their parent
# Rust module, but does NOT insert them into sys.modules. This means that
# `import ladle.io.bam` or `from ladle.ops.intervals import overlap` would raise
# ImportError at runtime and break IDE auto-import.
#
# This file manually registers every Rust submodule into sys.modules so that
# Python's import machinery can resolve the full dotted paths correctly.

import sys
import types

from . import _ladle_io   # noqa: F401
from . import _ladle_ops  # noqa: F401

# ── ladle.io ──────────────────────────────────────────────────────────────────

def _register(parent_name: str, parent_mod: types.ModuleType, children: dict) -> None:
    """Register *children* as submodules of *parent_mod* in sys.modules."""
    for name, mod in children.items():
        full = f"{parent_name}.{name}"
        sys.modules[full] = mod
        setattr(parent_mod, name, mod)

_io = types.ModuleType("ladle.io")
sys.modules["ladle.io"] = _io

_io_rust = _ladle_io.io
_register("ladle.io", _io, {
    "bam":   _io_rust.bam,
    "bcf":   _io_rust.bcf,
    "bgzf":  _io_rust.bgzf,
    "core":  _io_rust.core,
    "fastq": _io_rust.fastq,
    "sam":   _io_rust.sam,
    "vcf":   _io_rust.vcf,
})

# Leaf submodules two levels deep
_register("ladle.io.bam",  _io_rust.bam,  {"bai": _io_rust.bam.bai})
_register("ladle.io.bgzf", _io_rust.bgzf, {"gzi": _io_rust.bgzf.gzi})

# ── ladle.ops ─────────────────────────────────────────────────────────────────

_ops = types.ModuleType("ladle.ops")
sys.modules["ladle.ops"] = _ops

_intervals = types.ModuleType("ladle.ops.intervals")
_intervals.overlap        = _ladle_ops.overlap
_intervals.nearest        = _ladle_ops.nearest
_intervals.count_overlaps = _ladle_ops.count_overlaps
_intervals.cluster        = _ladle_ops.cluster
_intervals.merge          = _ladle_ops.merge
_intervals.subtract       = _ladle_ops.subtract
_intervals.complement     = _ladle_ops.complement
_intervals.coverage       = _ladle_ops.coverage
_intervals.expand         = _ladle_ops.expand
_intervals.shift          = _ladle_ops.shift
_intervals.sort_bedframe  = _ladle_ops.sort_bedframe
_intervals.flank          = _ladle_ops.flank
_intervals.set_width      = _ladle_ops.set_width
_intervals.tile           = _ladle_ops.tile
_intervals.disjoin        = _ladle_ops.disjoin
_register("ladle.ops", _ops, {"intervals": _intervals})

# ── Public API ────────────────────────────────────────────────────────────────

io  = _io
ops = _ops

__version__: str = _ladle_io.VERSION
__all__ = ["io", "ops", "__version__"]
