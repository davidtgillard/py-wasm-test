use wasmtime::{Engine, Linker, Module, Result, Store};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    p1::add_to_linker_sync(&mut linker, |t| t)?;

    let wasi_ctx = WasiCtxBuilder::new()
        .inherit_stdout()
        .args(&["python", "-c", "print('Hello from Python via Wasm!')"])
        .build_p1();

    let mut store = Store::new(&engine, wasi_ctx);

    let module = Module::from_file(&engine, "python-3.12.wasm")?;
    let instance = linker.instantiate(&mut store, &module)?;
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;

    Ok(())
}
