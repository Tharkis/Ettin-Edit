use std::path::Path;

fn main() {
    // Embed the icon and version info in the Windows executable.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/ettin-edit.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows icon: {e}");
        }
    }
    manual_pages();
}

/// Builds the user manual's pages into the app: every page named in
/// docs/manual/src/SUMMARY.md, so adding a page there is all it takes.
fn manual_pages() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/manual/src");
    println!("cargo:rerun-if-changed={}", src.display());
    let summary = src.join("SUMMARY.md");
    let text = std::fs::read_to_string(&summary).expect("docs/manual/src/SUMMARY.md");
    let mut code = format!(
        "/// docs/manual/src/SUMMARY.md: the manual's contents.\n\
         pub const SUMMARY: &str = include_str!({:?});\n\
         /// Every page of the manual: (file name, Markdown).\n\
         pub const PAGES: &[(&str, &str)] = &[\n",
        summary.display().to_string()
    );
    for line in text.lines() {
        let Some(start) = line.find("](") else {
            continue;
        };
        let rest = &line[start + 2..];
        let Some(end) = rest.find(')') else {
            continue;
        };
        let file = &rest[..end];
        if !file.ends_with(".md") {
            continue;
        }
        let path = src.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        code += &format!(
            "    ({file:?}, include_str!({:?})),\n",
            path.display().to_string()
        );
    }
    code += "];\n";
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("manual_pages.rs");
    std::fs::write(out, code).unwrap();
}
