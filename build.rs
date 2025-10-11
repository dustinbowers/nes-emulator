fn main() {
    // Allow overriding lib dirs via env if you built from source
    if let Ok(dir) = std::env::var("FREETYPE_DIR") {
        println!("cargo:rustc-link-search=native={}/lib", dir);
    }
    if let Ok(dir) = std::env::var("HARFBUZZ_DIR") {
        println!("cargo:rustc-link-search=native={}/lib", dir);
    }

    // Common Homebrew locations (Apple Silicon and Intel)
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    println!("cargo:rustc-link-search=native=/usr/local/lib");

    // Link the dependencies that SDL_ttf uses for glyph loading and shaping
    println!("cargo:rustc-link-lib=dylib=freetype");
    println!("cargo:rustc-link-lib=dylib=harfbuzz");

    println!("cargo:rustc-link-arg=-mmacosx-version-min=12.1");
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.1");
    println!("cargo:rerun-if-changed=build.rs");
}
