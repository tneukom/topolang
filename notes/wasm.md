Use `wasm-pack build --target web`, without the target it won't work, the js code tries
to load the wasm file as a module.

To run in rust-rover, open index.html and click one of the browser icons in 
top left of the editor. It will start a web server and open the file.