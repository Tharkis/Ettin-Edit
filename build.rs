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
    release_notes();
}

/// The release notes in the AppStream metainfo, as Markdown, for the
/// Welcome tab's What's new: `RELEASE_NOTES: &[(version, markdown)]`,
/// newest first.
fn release_notes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("packaging/linux/io.github.tharkis.EttinEdit.metainfo.xml");
    println!("cargo:rerun-if-changed={}", path.display());
    let xml = std::fs::read_to_string(&path).expect("metainfo.xml");
    let mut code = String::from(
        "/// The metainfo's release notes as Markdown: (version, notes), newest first.\n\
         pub const RELEASE_NOTES: &[(&str, &str)] = &[\n",
    );
    let mut rest = xml.as_str();
    while let Some(start) = rest.find("<release ") {
        rest = &rest[start..];
        let end = rest.find("</release>").unwrap_or(rest.len());
        let release = &rest[..end];
        rest = &rest[end..];
        let Some(version) = attr(release, "version") else {
            continue;
        };
        let mut notes = Vec::new();
        let mut body = release;
        loop {
            let p = body.find("<p>");
            let li = body.find("<li>");
            let (open, tag, close, bullet) = match (p, li) {
                (Some(p), Some(li)) if li < p => (li, "<li>", "</li>", true),
                (Some(p), _) => (p, "<p>", "</p>", false),
                (None, Some(li)) => (li, "<li>", "</li>", true),
                (None, None) => break,
            };
            body = &body[open + tag.len()..];
            let end = body.find(close).unwrap_or(body.len());
            let text = unescape(&body[..end].split_whitespace().collect::<Vec<_>>().join(" "));
            notes.push(if bullet {
                format!("- {text}")
            } else {
                format!("\n{text}\n")
            });
            body = &body[end..];
        }
        let markdown = notes.join("\n").trim().to_owned();
        code += &format!("    ({version:?}, {markdown:?}),\n");
    }
    code += "];\n";
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("release_notes.rs");
    std::fs::write(out, code).unwrap();
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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
