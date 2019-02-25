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

  println!("cargo:rustc-link-search=native=ptarmigan/btc");
  println!("cargo:rustc-link-search=native=ptarmigan/ln");
  println!("cargo:rustc-link-search=native=ptarmigan/utl");
  println!("cargo:rustc-link-search=native=ptarmigan/libs/install/lib");
  println!("cargo:rustc-link-lib=static=btc");
  println!("cargo:rustc-link-lib=static=ln");
  println!("cargo:rustc-link-lib=static=utl");
  println!("cargo:rustc-link-lib=static=mbedcrypto");
  println!("cargo:rustc-link-lib=static=stdc++");
}
