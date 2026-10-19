//! Windows: the icon and version details inside 2fip.exe (Explorer, the
//! taskbar, Properties → Details). Nothing to do on other systems.

fn main() {
    #[cfg(windows)]
    windows_resources();
}

#[cfg(windows)]
fn windows_resources() {
    println!("cargo:rerun-if-changed=../res/windows/2fip.ico");
    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon("../res/windows/2fip.ico")
        .set("ProductName", "2fip")
        .set("FileDescription", "2fip file manager")
        .set("CompanyName", "chillalabs")
        .set("LegalCopyright", "GPL-3.0-or-later");
    resources
        .compile()
        .expect("failed to add the icon and version details to 2fip.exe");
}
