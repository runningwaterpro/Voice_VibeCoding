fn main() {
    // Tauri's default Windows app manifest is embedded in the app binary, but
    // cargo's library-test harness does not receive that resource. Dialog code
    // imports TaskDialogIndirect from Common Controls v6, so the test binary
    // otherwise fails before the first test with STATUS_ENTRYPOINT_NOT_FOUND.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
    tauri_build::build()
}
