use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("build_time.txt");
    
    let build_time = chrono::Utc::now().to_rfc3339();
    fs::write(&dest_path, &build_time).unwrap();
    
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    
    if let Ok(hash) = env::var("GIT_HASH") {
        println!("cargo:rustc-env=GIT_HASH={}", hash);
    } else {
        println!("cargo:rustc-env=GIT_HASH=unknown");
    }
}
