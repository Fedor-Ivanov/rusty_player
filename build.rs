//! Embeds the icon into the .exe when building for Windows (including cross-builds from macOS via MinGW).

fn main() {
    println!("cargo:rerun-if-changed=assets/images/app_icon.ico");

    // build.rs runs on the build machine, so check the target OS rather than cfg!(windows)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/images/app_icon.ico");
        resource
            .compile()
            .expect("could not embed the icon into the .exe");
    }
}
