fn main() {
    // This build script ensures the WIT interface is available
    // and sets up the WASM component properly
    
    // Generate WIT bindings at build time
    wit_bindgen::generate!({
        path: "../../crates/gromnie-scripting-api/src/wit",
        world: "script",
    });
}