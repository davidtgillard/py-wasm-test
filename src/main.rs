use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use tempfile::NamedTempFile;
use wasmtime::component::{bindgen, Component};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

// 1. Generate bindings from the WIT file
bindgen!("adder" in "adder.wit");

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

/// Build the Python→Wasm component programmatically using componentize-py.
/// `project_root` must be the directory containing adder.wit, app.py, wit_world/, etc.
async fn build_component_async(project_root: &Path) -> Result<Vec<u8>> {
    let wit_path = [project_root];
    let world = Some("adder");
    let features: &[String] = &[];
    let all_features = false;
    let world_module: Option<&str> = None;
    let python_path = [project_root.to_str().expect("project root is valid UTF-8")];
    let module_worlds: &[(&str, &str)] = &[];
    let app_name = "app";
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
    let component_bytes = build_component_async(&project_root).await?;
    let component = Component::new(&engine, &component_bytes)?;

    let adder = Adder::instantiate(&mut store, &component, &linker)?;

    //let bytes = b"";
    let bytes = b"hello, component";
    let result = adder.call_add(&mut store, bytes)?;
    match result {
        AddResultOrError::Ok(ok) => {
            println!("length: {}, data: {:?}", ok.length, ok.data);
        }
        AddResultOrError::Err(msg) => {
            eprintln!("error: {}", msg);
        }
    }

    Ok(())
}