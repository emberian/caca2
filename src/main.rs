#[cfg(not(target_arch = "wasm32"))]
fn main() {
    caca::main();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
