use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    // Default: vendor and statically link SDL2 + SDL2_ttf.
    // Opt-in to system libraries only if NES_USE_SYSTEM_SDL=1 is set in the environment.
    if env::var("NES_USE_SYSTEM_SDL").ok().as_deref() == Some("1") {
        if pkg_config::Config::new().probe("sdl2").is_ok() {
            println!("cargo:rerun-if-env-changed=NES_USE_SYSTEM_SDL");
            println!("cargo:rerun-if-env-changed=SDL2");
            println!("cargo:rerun-if-env-changed=SDL2_TTF");
            println!("cargo:rustc-link-arg=-mmacosx-version-min=12.1");
            let _ = pkg_config::Config::new().probe("SDL2_ttf");
            return;
        }
        // Fall through to vendored if system SDL2 not found
    }

    println!("System SDL2 not found. Building SDL2 and SDL2_ttf from source...");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // Ensure both CMake and Rust use the same macOS deployment target
    env::set_var("MACOSX_DEPLOYMENT_TARGET", "12.1");
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.1");
    let sdl_dir = out_dir.join("SDL2");

    // If SDL2 hasn’t been downloaded yet, fetch it
    if !sdl_dir.exists() {
        fs::create_dir_all(&sdl_dir).unwrap();

        let url = "https://github.com/libsdl-org/SDL/archive/refs/tags/release-2.30.7.zip";
        let zip_path = out_dir.join("sdl2.zip");

        // Download SDL2 source (curl or wget)
        Command::new("curl")
            .args(["-L", "-o", zip_path.to_str().unwrap(), url])
            .status()
            .expect("Failed to download SDL2");

        // Unzip it
        Command::new("unzip")
            .args([zip_path.to_str().unwrap(), "-d", sdl_dir.to_str().unwrap()])
            .status()
            .expect("Failed to unzip SDL2 source");
    }

    // Find inner folder (SDL-release-2.30.7)
    let source_dir = fs::read_dir(&sdl_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();

    let dst = cmake::Config::new(&source_dir)
        .define("SDL_STATIC", "ON")
        .define("SDL_SHARED", "OFF")
        .define("SDL_TEST", "OFF")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_OSX_DEPLOYMENT_TARGET", "12.1")
        .build();

    let lib_dir = dst.join("lib");
    let include_dir = dst.join("include");

    // Tell cargo where to find SDL2 static lib and headers
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // Force static by linking the exact archive filename
    println!("cargo:rustc-link-lib=static=:libSDL2.a");
    println!("cargo:include={}", include_dir.display());






    // Download and build SDL2_ttf
    let ttf_url = "https://github.com/libsdl-org/SDL_ttf/archive/refs/tags/release-2.20.2.zip";
    let ttf_zip_path = out_dir.join("sdl2_ttf.zip");
    let ttf_dir = out_dir.join("SDL2_ttf");

    if !ttf_dir.exists() {
        fs::create_dir_all(&ttf_dir).unwrap();
        Command::new("curl")
            .args(["-L", "-o", ttf_zip_path.to_str().unwrap(), ttf_url])
            .status()
            .expect("Failed to download SDL2_ttf");
        Command::new("unzip")
            .args([ttf_zip_path.to_str().unwrap(), "-d", ttf_dir.to_str().unwrap()])
            .status()
            .expect("Failed to unzip SDL2_ttf source");
    }

    let ttf_source_dir = fs::read_dir(&ttf_dir)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();

    let ttf_dst = cmake::Config::new(&ttf_source_dir)
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_OSX_DEPLOYMENT_TARGET", "12.1")
        // Prefer static build
        .define("BUILD_SHARED_LIBS", "OFF")
        // Ask SDL_ttf to use vendored dependencies where possible (e.g., FreeType)
        .define("SDL2TTF_VENDORED", "ON")
        // Help CMake find the SDL2 we just built
        .define("CMAKE_PREFIX_PATH", dst.to_str().unwrap())
        .define("SDL2_DIR", dst.join("lib/cmake/SDL2").to_str().unwrap())
        .build();

    let ttf_lib_dir = ttf_dst.join("lib");
    let ttf_include_dir = ttf_dst.join("include");

    println!("cargo:rustc-link-search=native={}", ttf_lib_dir.display());
    println!("cargo:include={}", ttf_include_dir.display());
    // Force static by linking the exact archive filename
    println!("cargo:rustc-link-lib=static=:libSDL2_ttf.a");
    for lib in ["freetype", "png", "z", "bz2", "harfbuzz"] {
        let candidate = format!("lib{}.a", lib);
        if ttf_lib_dir.join(&candidate).exists() {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }







    // Re-run if this script or SDL2 version changes
    println!("cargo:rerun-if-changed=build.rs");
}
