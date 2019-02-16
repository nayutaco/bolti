use std::env;
use std::path::PathBuf;

fn main() {
  let bindings = bindgen::Builder::default()
    .header("src/bindings.h")
    .clang_arg("-Iptarmigan/utl")
    .clang_arg("-Iptarmigan/btc")
    .generate()
    .expect("Unable to generate bindings");

  // Cargo sets $OUT_DIR
  let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
  bindings
    .write_to_file(out_path.join("bindings.rs"))
    .expect("Couldn't write bindings!");

  cc::Build::new()
    .file("ptarmigan/ln/ln.c")
    .file("ptarmigan/ln/ln_noise.c")
    .flag("-Iptarmigan/utl")
    .flag("-Iptarmigan/btc")
    .compile("ptarmigan");
}
