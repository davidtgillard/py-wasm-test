use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;
use wasmtime::component::{bindgen, Component};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// Component/world name (module and directory name; WIT file is COMPONENT_NAME.wit).
const COMPONENT_NAME: &str = "length_calc";
/// World name as in the WIT file (kebab-case). Must match the `world` definition in the .wit file.
const WORLD_NAME_WIT: &str = "length-calc";
/// Python entry module name (entry point is APP_NAME.py).
const APP_NAME: &str = "app";

// 1. Generate bindings from the WIT file (world name must match WIT; file = COMPONENT_NAME.wit)
bindgen!("length-calc" in "length_calc.wit");

struct MyState {
    ctx: WasiCtx,
    table: ResourceTable,
}
impl WasiView for MyState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

fn cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(".component_cache")
}

fn cache_component_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(format!("{}.component.wasm", COMPONENT_NAME))
}

fn cache_fingerprint_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(".fingerprint")
}

fn cache_precompiled_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(format!("{}.precompiled", COMPONENT_NAME))
}

/// Collect paths that affect the component: COMPONENT_NAME.wit, APP_NAME.py, and COMPONENT_NAME/** (skip __pycache__, .pyc).
fn component_input_paths(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let wit = project_root.join(format!("{}.wit", COMPONENT_NAME));
    let app = project_root.join(format!("{}.py", APP_NAME));
    if wit.exists() {
        paths.push(wit);
    }
    if app.exists() {
        paths.push(app);
    }
    let component_dir = project_root.join(COMPONENT_NAME);
    if component_dir.is_dir() {
        for entry in fs::read_dir(&component_dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.file_name().and_then(|n| n.to_str()) == Some("__pycache__") {
                continue;
            }
            if p.extension().and_then(|e| e.to_str()) == Some("pyc") {
                continue;
            }
            if p.is_dir() {
                for sub in fs::read_dir(&p)? {
                    let sub = sub?;
                    paths.push(sub.path());
                }
            } else {
                paths.push(p);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

/// Content-based fingerprint of inputs (COMPONENT_NAME.wit, APP_NAME.py, COMPONENT_NAME/**). Returns hex string.
fn compute_component_fingerprint(project_root: &Path) -> Result<String> {
    let paths = component_input_paths(project_root)?;
    let mut hasher = blake3::Hasher::new();
    for path in paths {
        let contents = fs::read(&path)?;
        let file_hash = blake3::hash(&contents);
        let path_str = path.to_string_lossy();
        hasher.update(path_str.as_bytes());
        hasher.update(file_hash.as_bytes());
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Build the Python→Wasm component programmatically using componentize-py.
/// `project_root` must be the directory containing COMPONENT_NAME.wit, APP_NAME.py, COMPONENT_NAME/, etc.
async fn build_component_async(project_root: &Path) -> Result<Vec<u8>> {
    let wit_path = [project_root];
    let world = Some(WORLD_NAME_WIT);
    let features: &[String] = &[];
    let all_features = false;
    let world_module: Option<&str> = Some(COMPONENT_NAME);
    let python_path = [project_root.to_str().expect("project root is valid UTF-8")];
    let module_worlds: &[(&str, &str)] = &[];
    let app_name = APP_NAME;
    let tmp = NamedTempFile::new()?;
    let output_path = tmp.path();
    let stub_wasi = false;
    let import_interface_names = HashMap::new();
    let export_interface_names = HashMap::new();

    componentize_py::componentize(
        &wit_path,
        world,
        features,
        all_features,
        world_module,
        &python_path,
        module_worlds,
        app_name,
        output_path,
        None,
        stub_wasi,
        &import_interface_names,
        &export_interface_names,
    )
    .await?;

    let mut bytes = Vec::new();
    std::fs::File::open(output_path)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Load component from cache if fingerprint matches; otherwise build and write cache.
/// Set PY_WASM_REBUILD=1 to force rebuild and refresh cache.
async fn get_or_build_component(project_root: &Path) -> Result<Vec<u8>> {
    let force_rebuild = std::env::var("PY_WASM_REBUILD")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let fingerprint = compute_component_fingerprint(project_root)?;
    let cache_dir = cache_dir(project_root);
    let component_path = cache_component_path(&cache_dir);
    let fingerprint_path = cache_fingerprint_path(&cache_dir);

    if !force_rebuild
        && fingerprint_path.exists()
        && component_path.exists()
    {
        let cached = fs::read_to_string(&fingerprint_path).ok();
        if cached.as_deref() == Some(fingerprint.as_str()) {
            if let Ok(bytes) = fs::read(&component_path) {
                return Ok(bytes);
            }
        }
    }

    let bytes = build_component_async(project_root).await?;
    fs::create_dir_all(&cache_dir)?;
    fs::write(&component_path, &bytes)?;
    fs::write(&fingerprint_path, &fingerprint)?;
    Ok(bytes)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true); // Enable component model
    let engine = Engine::new(&config)?;

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    let mut store = Store::new(
        &engine,
        MyState {
            ctx: WasiCtxBuilder::new().inherit_stdout().build(),
            table: ResourceTable::new(),
        },
    );

    let project_root = std::env::current_dir()?;
    let component_bytes = get_or_build_component(&project_root).await?;

    let force_rebuild = std::env::var("PY_WASM_REBUILD")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let fingerprint = compute_component_fingerprint(&project_root)?;
    let cache_dir = cache_dir(&project_root);
    let fingerprint_path = cache_fingerprint_path(&cache_dir);
    let precompiled_path = cache_precompiled_path(&cache_dir);

    let component = {
        let try_precompiled = !force_rebuild
            && fingerprint_path.exists()
            && precompiled_path.exists()
            && fs::read_to_string(&fingerprint_path).ok().as_deref() == Some(fingerprint.as_str());

        if try_precompiled {
            if let Ok(precompiled_bytes) = fs::read(&precompiled_path) {
                // SAFETY: Bytes were produced by Component::serialize() from this binary
                // with the same Engine config; we do not modify them.
                if let Ok(c) = unsafe { Component::deserialize(&engine, &precompiled_bytes) } {
                    c
                } else {
                    let c = Component::new(&engine, &component_bytes)?;
                    fs::create_dir_all(&cache_dir)?;
                    let _ = fs::write(&precompiled_path, &c.serialize()?);
                    c
                }
            } else {
                let c = Component::new(&engine, &component_bytes)?;
                fs::create_dir_all(&cache_dir)?;
                let _ = fs::write(&precompiled_path, &c.serialize()?);
                c
            }
        } else {
            let c = Component::new(&engine, &component_bytes)?;
            fs::create_dir_all(&cache_dir)?;
            let _ = fs::write(&precompiled_path, &c.serialize()?);
            c
        }
    };

    let length_calc = LengthCalc::instantiate(&mut store, &component, &linker)?;

    //let bytes = b"";
    let bytes = b"hello, component";
    let result = length_calc.call_length_calc(&mut store, bytes)?;
    match result {
        LengthCalcResultOrError::Ok(ok) => {
            println!("length: {}, data: {:?}", ok.length, ok.data);
        }
        LengthCalcResultOrError::Err(msg) => {
            eprintln!("error: {}", msg);
        }
    }

    Ok(())
}