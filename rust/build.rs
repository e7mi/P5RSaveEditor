fn main() {
    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("assets/icon.ico");
        resource.set("ProductName", "P5R Save Editor");
        resource.set("FileDescription", "Persona 5 Royal save editor");
        resource.set("OriginalFilename", "P5RSaveEditor.exe");
        if let Ok(version) = std::env::var("CARGO_PKG_VERSION") {
            resource.set("FileVersion", &version);
            resource.set("ProductVersion", &version);
        }
        if let Err(error) = resource.compile() {
            println!("cargo:warning=winres: {error}");
        }
    }
}
