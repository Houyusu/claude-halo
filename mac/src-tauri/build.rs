fn main() {
    // CoreGraphics provides CGWindowListCopyWindowInfo and the
    // kCGWindow* CFString constants used by platform.rs for
    // window-level focus detection.
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    tauri_build::build()
}
