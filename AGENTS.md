# ladle — Agent Guide

**ladle** is a Python binding to the Rust `noodles` bioinformatics library via PyO3.
The goal is to mirror noodles' public API exactly, with a columnar layer on top via Arrow/Polars.

---

## Project layout

```
ladle/
├── Cargo.toml              workspace (members: ladle-io, ladle-ops)
├── pyproject.toml          maturin + uv, Python 3.11, optional deps
├── python/ladle/
│   ├── __init__.py         sys.modules registration for ladle.io.* and ladle.ops.*
│   ├── io/                 hand-written .pyi stubs
│   └── ops/
├── ladle-io/               I/O formats (SAM, BAM, VCF, BCF, BGZF, core)
│   └── src/
│       ├── lib.rs
│       ├── arrow_utils.rs  shared Arrow C Data Interface helpers
│       └── io/
│           ├── bam/        reader.rs, writer.rs, record.rs, batch.rs, bai.rs
│           ├── bcf/        reader.rs, writer.rs, record.rs, batch.rs
│           ├── bgzf/       reader.rs, writer.rs, virtual_position.rs, gzi.rs
│           ├── core.rs     Position, Interval, Region
│           ├── sam/        reader.rs, writer.rs, record.rs, batch.rs, flags.rs,
│           │               header.rs, mapping_quality.rs
│           └── vcf/        reader.rs, writer.rs, record.rs, batch.rs, header.rs,
│                           schema.rs
├── ladle-ops/              algorithms (interval overlap / nearest)
│   └── src/
│       ├── lib.rs
│       ├── arrow_utils.rs  (copy of ladle-io version — intentional duplication)
│       └── intervals/      ops.rs, schema.rs
└── tests/                  pytest, ~340 test cases
```

---

## Build and test

```bash
# one-time setup (use pip in .venv, not uv — corporate TLS blocks uv→PyPI)
python3.11 -m venv .venv
.venv/bin/pip install maturin pytest pytest-cov ruff pyarrow polars pandas

# build both crates and install into .venv
VIRTUAL_ENV="$(pwd)/.venv" .venv/bin/maturin develop --manifest-path ladle-io/Cargo.toml
VIRTUAL_ENV="$(pwd)/.venv" .venv/bin/maturin develop --manifest-path ladle-ops/Cargo.toml

# run tests
.venv/bin/pytest
```

After any Rust change, re-run `maturin develop` for the affected crate before running tests.

### Validation commands for agents

After every code change, run both commands to validate correctness and style:

```bash
just test   # builds both crates, runs pytest + cargo test
just lint   # ruff check on Python, cargo clippy on Rust (warnings → errors)
```

`just test` already rebuilds the Rust extensions before running tests, so there is no need to call `just build` separately. Fix all errors and warnings from both commands before reporting the work as done.

### Keeping Python stubs in sync with Rust

Every `#[pyfunction]` signature in `ladle-ops/src/intervals/ops.rs` and every `#[pymethods]` block in `ladle-io/src/` must have a matching entry in the corresponding `.pyi` stub under `python/ladle/`.

After adding or changing a Rust binding, manually verify the stub matches on all of:

| Rust | Python stub |
|---|---|
| parameter name | parameter name |
| parameter type (`Option<Vec<String>>` → `list[str] \| None`) | parameter type |
| default value (`#[pyo3(signature = (...))]`) | default value |
| return type | return type |

Use `Literal[...]` for string parameters whose valid values are checked at runtime (e.g. `how`, `anchor`). Use `list[str] \| None = None` for optional `Vec<String>` parameters.

The stubs live at:
- `python/ladle/io/<format>.pyi` — one file per I/O format
- `python/ladle/ops/intervals.pyi` — all interval operations

---

## Core design rules

### Mirror noodles exactly

Every Python class, method name, and field name matches its noodles counterpart.
No convenience constructors beyond what noodles offers.
A user who knows the noodles Rust docs should write identical Python.

### Type erasure over explicit monomorphisation

noodles readers are generic (`Reader<R: Read>`). Python isn't.
All readers use `Box<dyn BufRead + Send>` or a concrete monomorphised alias rather than
exposing multiple `BamFileReader` / `BamBgzfReader` variants.

### `#[pyclass]` wrapper pattern

```rust
#[pyclass(name = "Reader", module = "ladle.bam")]
pub struct PyReader {
    inner: Option<InnerReader>,  // None means closed
}
```

`Option<Inner>` enables the `close()` / context-manager pattern without unsafe.
Always call `self.get()` to unwrap; it returns `PyIOError` if closed.

### from_fd (Unix-only)

BAM and VCF readers already have `from_fd`. The pattern is:

```rust
#[cfg(unix)]
#[staticmethod]
fn from_fd(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
    let fd: RawFd = obj
        .call_method0("fileno")
        .map_err(|_| PyIOError::new_err("fd object must have a fileno() method"))?
        .extract()?;
    let owned_fd = unsafe { libc::dup(fd) };   // Rust owns a dup'd fd
    if owned_fd < 0 {
        return Err(PyIOError::new_err(std::io::Error::last_os_error().to_string()));
    }
    let file = unsafe { File::from_raw_fd(owned_fd) };
    // wrap in BufReader for text formats (SAM/VCF/BCF); use directly for BGZF formats (BAM/BCF)
    let inner = noodles::FORMAT::io::Reader::new(BufReader::new(file));
    Ok(Self { inner: Some(inner) })
}
```

`libc` is already a dependency of `ladle-io`. BCF wraps BGZF so no `BufReader`
(same as BAM). SAM needs `BufReader` (same as VCF).

### Arrow integration

`arrow_utils.rs` in each crate provides three shared helpers:

| Function | Direction |
|---|---|
| `pyarrow_to_batch(py, obj)` | Python (`pyarrow.RecordBatch`, `polars.DataFrame`, `__arrow_c_stream__`) → Rust `RecordBatch` |
| `batch_to_pyarrow(py, batch)` | Rust `RecordBatch` → `pyarrow.RecordBatch` (zero-copy via C Stream Interface) |
| `pandas_to_batch(py, obj)` | `pandas.DataFrame` → Rust `RecordBatch` (via pyarrow) |

`RecordBatch.to_iterator()` only works on batches created from a reader, not from
`from_arrow`/`from_polars`/`from_pandas` — the raw noodles records are required and
not reconstructible from Arrow columns.

### GIL strategy

- Release GIL during batch reads: `py.detach(|| { /* read all records */ })`
- Keep GIL when constructing Python objects (`PyRecord::from(...)`)
- `IndexedReader` and `PyIndexedReader` use `unsafe impl Send` because `BinningIndex`
  is not `Send`, but PyO3 holds the GIL on every method call, so single-threaded access is guaranteed.

### sys.modules registration

PyO3 `#[pymodule(submodule)]` does not add submodules to `sys.modules`.
`python/ladle/__init__.py` manually registers every Rust submodule using `_register()`.
When adding a new format, update `__init__.py` to register it under `ladle.io.<format>`.

---

## Module status

| Module | Reader | Writer | Record | RecordBatch | from_fd | IndexedReader |
|---|---|---|---|---|---|---|
| `ladle.io.core` | — | — | Position, Interval, Region | — | — | — |
| `ladle.io.bgzf` | ✓ | ✓ | VirtualPosition, gzi.Index | — | — | — |
| `ladle.io.fastq` | ✓ | ✓ | ✓ | — | ✓ | — |
| `ladle.io.sam` | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `ladle.io.bam` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `ladle.io.vcf` | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `ladle.io.bcf` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `ladle.ops.intervals` | — | — | — | overlap, nearest | — | — |

---

## RecordBatch schemas

### BAM (`ladle.io.bam.RecordBatch`)

| Column | Arrow type | Nullable |
|---|---|---|
| `name` | `LargeBinary` | yes |
| `flags` | `UInt16` | no |
| `reference_sequence_id` | `Int32` | yes |
| `alignment_start` | `Int32` | yes |
| `mapping_quality` | `UInt8` | yes |
| `cigar` | `LargeBinary` | no |
| `mate_reference_sequence_id` | `Int32` | yes |
| `mate_alignment_start` | `Int32` | yes |
| `template_length` | `Int32` | no |
| `sequence` | `LargeBinary` | no |
| `quality_scores` | `LargeBinary` | no |

SAM uses `LargeUtf8` for `name`, `cigar`, `sequence`; quality is ASCII phred text.
VCF/BCF base columns: `chrom`, `pos`, `id`, `ref`, `alt`, `qual` + optional INFO/FORMAT columns from header.

---

## Intervals (`ladle.ops.intervals`)

```python
result = intervals.overlap(a, b)          # RecordBatch / polars.DataFrame / pandas.DataFrame
result = intervals.nearest(query, target) # same input types
```

- Columns resolved automatically: `chrom`/`contig`/`chr`, `start`/`pos`, `end`/`stop`
- Parallel by chromosome via Rayon + COITree
- `overlap` output: all `a_*` and `b_*` prefixed columns, one row per overlap pair
- `nearest` output: same length as `query`, includes `distance` column (`Int64`, nullable for no match)
- GIL released with `py.detach()` during parallel work

---

## Adding a new format

1. Create `ladle-io/src/io/<format>/` with `mod.rs`, `reader.rs`, `writer.rs`, `record.rs`.
2. Register the module in `ladle-io/src/io/mod.rs`.
3. Add to `python/ladle/__init__.py` under `_register("ladle.io", _io, {...})`.
4. Add a `.pyi` stub in `python/ladle/io/<format>.pyi`.
5. Write tests in `tests/test_<format>.py`.

For formats with BGZF compression (BCF-style), use `noodles::bgzf::io::Reader<File>` directly
(no `BufReader`). For plain text formats (SAM/VCF-style), wrap with `BufReader<File>`.

---

## Key dependencies

| Crate | Version | Purpose |
|---|---|---|
| `pyo3` | 0.28 | Python ↔ Rust FFI |
| `noodles` | 0.109 | bioinformatics I/O |
| `arrow` / `arrow-array` | 58 | columnar data |
| `bstr` | 1 | byte-string utilities |
| `libc` | latest | `dup()` for `from_fd` |
| `coitrees` | 0.4 | interval tree (ladle-ops) |
| `rayon` | 1 | parallelism (ladle-ops) |
