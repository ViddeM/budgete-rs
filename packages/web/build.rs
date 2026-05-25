fn main() {
    // Re-run whenever any SCSS source changes.
    println!("cargo:rerun-if-changed=assets/main.scss");
    println!("cargo:rerun-if-changed=assets/styles/");

    let scss_src = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/main.scss");
    let css_out = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/main.css");

    let opts = grass::Options::default()
        .style(grass::OutputStyle::Compressed)
        .load_path(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"));

    let css = grass::from_path(scss_src, &opts).expect("SCSS compilation failed");

    std::fs::write(css_out, css).expect("failed to write main.css");
}
