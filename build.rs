fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        println!("Building for Windows...");
        let mut res = winres::WindowsResource::new();
        res.set_icon("my_icon.ico");
        res.compile().unwrap();
    }
}
