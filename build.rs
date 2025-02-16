#[cfg(windows)]
extern crate winresource;

#[cfg(windows)]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("release/yer.ico");
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
