fn main() {

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/rektal-logo-without-title-32px.ico");
        res.compile().unwrap();
    }
}
