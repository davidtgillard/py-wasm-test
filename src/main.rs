use wasmtime::{Config, Engine, Result, Store};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime::component::{Component, bindgen};

// 1. Generate bindings from the WIT file
bindgen!("adder" in "adder.wit");

struct MyState {
    ctx: WasiCtx,
    table: ResourceTable,
}
impl WasiView for MyState {
    fn ctx(&mut self) -> WasiCtxView<'_> { 
        WasiCtxView { ctx: &mut self.ctx, table: &mut self.table } 
    }
}


fn main() -> Result<()> {

    let mut config = Config::new();
    config.wasm_component_model(true); // Enable component model
    let engine = Engine::new(&config)?;

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;

    let mut store = Store::new(&engine, MyState {
        ctx: WasiCtxBuilder::new().inherit_stdout().build(),
        table: ResourceTable::new(),
    });

    let component = Component::from_file(&engine, "adder.component.wasm")?;

    let adder = Adder::instantiate(&mut store, &component, &linker)?;

    let bytes = b"hello, component";
    let result = adder.call_add(&mut store, bytes)?;
    println!("length: {}, data: {:?}", result.length, result.data);

    Ok(())
}

/*

use wasmtime::component::{bindgen, Component, Linker};
use wasmtime::{Config, Engine, Result, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, ResourceTable, WasiView};

// 1. Generate bindings from the WIT file
bindgen!("adder" in "adder.wit");

struct MyState {
    ctx: WasiCtx,
    table: ResourceTable,
}
impl WasiView for MyState {
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

fn main() -> Result<()> {
    let mut config = Config::new();
    config.wasm_component_model(true); // Enable component model
    let engine = Engine::new(&config)?;

    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    let mut store = Store::new(&engine, MyState {
        ctx: WasiCtxBuilder::new().inherit_stdout().build(),
        table: ResourceTable::new(),
    });

    // 2. Load the COMPONENT produced by componentize-py
    let component = Component::from_file(&engine, "adder.component.wasm")?;

    // 3. Instantiate using the generated bindings
    let (adder, _) = Adder::instantiate(&mut store, &component, &linker)?;

    // 4. Call the Python function directly
    let result = adder.call_add(&mut store, 5, 10)?;
    println!("Result from Python: {}", result);

    Ok(())
}*/