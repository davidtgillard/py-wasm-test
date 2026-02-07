# py-wasm-test

## Description

A small demo that runs **Python inside WebAssembly** using the [Wasm component model](https://component-model.bytecodealliance.org/). A Rust host uses [Wasmtime](https://wasmtime.dev/) to build a Python component with [componentize-py](https://github.com/bytecodealliance/componentize-py), then instantiates and calls it.

- **WIT** (`length_calc.wit`) defines a world with a `length_calc` function: bytes in → `{ length, data }` or error.
- **Python** (`length_calc.py`) implements that interface; componentize-py compiles it to a `.wasm` component.
- **Rust** (`src/main.rs`) builds (or loads from cache) the component, instantiates it, and invokes `length_calc` with sample data.

The host caches the built component under `.component_cache/` using a content fingerprint (length_calc.wit, length_calc.py, length_calc/**), and can cache a precompiled component for faster startup.

## Motivation

- See how to **embed Python logic in a Rust application** via Wasm components.
- Exercise **componentize-py** and Wasmtime’s component model in a minimal, runnable example.
- Validate a **build-and-cache** workflow so repeated runs skip rebuilding when sources are unchanged.

## Usage

**Prerequisites:** Rust toolchain, Python 3 (used by componentize-py when building).

From the project root (`py-wasm-test/`):

```bash
cargo run
```

On first run this builds the Python→Wasm component (and may take a while); later runs use the cache unless sources change.

To force a clean rebuild and refresh the cache:

```bash
PY_WASM_REBUILD=1 cargo run
```

Expected output is something like: `length: 16, data: [...]` for the sample input `b"hello, component"`.
