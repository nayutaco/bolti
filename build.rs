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
    .file("ptarmigan/ln/ln_node.c")
    .file("ptarmigan/utl/utl_buf.c")
    .file("ptarmigan/utl/utl_log.c")
    .file("ptarmigan/utl/utl_time.c")
    .file("ptarmigan/utl/utl_str.c")
    .file("ptarmigan/btc/btc_keys.c")
    .file("ptarmigan/btc/btc_crypto.c")
    .file("ptarmigan/libs/mbedtls/library/chachapoly.c")
    .file("ptarmigan/libs/mbedtls/library/bignum.c")
    .file("ptarmigan/libs/mbedtls/library/ecp.c")
    .file("ptarmigan/libs/mbedtls/library/poly1305.c")
    .file("ptarmigan/libs/mbedtls/library/chacha20.c")
    .file("ptarmigan/libs/mbedtls/library/md.c")
    .file("ptarmigan/libs/mbedtls/library/hkdf.c")
    .file("ptarmigan/libs/mbedtls/library/sha256.c")
    .file("ptarmigan/libs/mbedtls/library/ecp_curves.c")
    .file("ptarmigan/libs/mbedtls/library/ctr_drbg.c")
    .file("ptarmigan/libs/mbedtls/library/error.c")
    .file("ptarmigan/libs/mbedtls/library/platform.c")
    .file("ptarmigan/libs/mbedtls/library/platform_util.c")
    .file("ptarmigan/libs/mbedtls/library/aes.c")
    .file("ptarmigan/libs/mbedtls/library/md5.c")
    .file("ptarmigan/libs/mbedtls/library/md_wrap.c")
    .file("ptarmigan/libs/mbedtls/library/sha1.c")
    .file("ptarmigan/libs/mbedtls/library/sha512.c")
    .file("ptarmigan/libs/mbedtls/library/aesni.c")
    .file("ptarmigan/libs/mbedtls/library/ripemd160.c")
    .file("ptarmigan/libs/mbedtls/library/entropy.c")
    .file("ptarmigan/libs/mbedtls/library/entropy_poll.c")
    .file("ptarmigan/libs/mbedtls/library/timing.c")
    .include("ptarmigan/utl")
    .include("ptarmigan/btc")
    .include("ptarmigan/libs/install/include")
    .compile("ptarmigan");
}
