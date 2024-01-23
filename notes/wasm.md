Use `wasm-pack build --target web`, without the target it won't work, the js code tries
to load the wasm file as a module.