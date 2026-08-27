//! Android business kernels. Both kernels share the same input shape:
//! a config JSON path plus the target project folder.
//!
//! - [`argument`] — `write_mode == "argument"` entries: resolve values
//!   (override/prefix/format) and inject them into project files or copy
//!   files (Rust port of `argument_kernel.py`).
//! - [`code`] — `write_mode == "code"` entries: generate Java methods
//!   from scene definitions and inject them between area markers
//!   (Rust port of `code_kernel.py`).

pub mod argument;
pub mod code;
