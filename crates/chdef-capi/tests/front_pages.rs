//! Every published artifact carries a front page written for the reader it
//! is shown to.
//!
//! 0.0.12 moved the NuGet page to one written for a C# reader and then
//! packed the repository readme anyway, so nuget.org kept showing a page
//! with no C# on it. The test written to prevent that asserted the project
//! *declares* a readme — true, and true while the wrong file was packed.
//!
//! What has to be checked is which file each package carries and whether
//! it addresses the right reader. All three are checked here, together,
//! because the defect was not specific to one of them.

use std::path::Path;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The value of a `key = "…"` line of a manifest.
fn manifest_value(manifest: &str, key: &str) -> String {
    manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix(&format!("{key} = \"")))
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_else(|| panic!("no {key} in the manifest"))
        .to_string()
}

/// How many fenced blocks of `language` a page has.
fn blocks(page: &str, language: &str) -> usize {
    page.lines()
        .filter(|line| line.trim() == format!("```{language}"))
        .count()
}

#[test]
fn each_crate_carries_the_page_beside_it() {
    // A readme outside the crate directory is also what `cargo package`
    // cannot include from `include_str!`, so this keeps two problems shut.
    for crate_dir in ["crates/chdef", "crates/chdef-capi"] {
        let manifest = read(&format!("{crate_dir}/Cargo.toml"));
        assert_eq!(
            manifest_value(&manifest, "readme"),
            "README.md",
            "{crate_dir} points its front page somewhere else"
        );
        assert!(
            !read(&format!("{crate_dir}/README.md")).is_empty(),
            "{crate_dir} has no front page"
        );
    }
}

#[test]
fn the_dotnet_package_carries_the_page_beside_it() {
    let project = read("bindings/dotnet/Chdef/Chdef.csproj");

    assert!(
        project.contains("<PackageReadmeFile>README.md</PackageReadmeFile>"),
        "the package declares no readme"
    );

    let packed = project
        .lines()
        .filter(|line| line.contains("<None Include=") && line.contains("Pack=\"true\""))
        .filter_map(|line| line.split("Include=\"").nth(1))
        .filter_map(|rest| rest.split('"').next())
        .find(|path| path.to_lowercase().ends_with("readme.md"))
        .expect("nothing named readme.md is packed");

    assert_eq!(
        packed, "README.md",
        "the package must carry the page beside it; {packed} is somewhere else"
    );
}

#[test]
fn each_page_speaks_the_language_of_its_reader() {
    let rust = read("crates/chdef/README.md");
    assert!(blocks(&rust, "rust") >= 3, "the crate page shows no Rust");
    assert_eq!(blocks(&rust, "csharp"), 0, "the crate page shows C#");

    let c = read("crates/chdef-capi/README.md");
    assert!(blocks(&c, "c") >= 1, "the C ABI page shows no C");
    assert_eq!(blocks(&c, "rust"), 0, "the C ABI page shows Rust");
    assert!(
        c.contains("dotnet add package Chdef"),
        "the C ABI page does not send a C# reader to the package that suits them"
    );

    let csharp = read("bindings/dotnet/Chdef/README.md");
    assert!(blocks(&csharp, "csharp") >= 4, "the NuGet page shows no C#");
    assert_eq!(blocks(&csharp, "rust"), 0, "the NuGet page shows Rust");
    assert!(
        csharp.contains("dotnet add package Chdef"),
        "the NuGet page does not say how to install it"
    );

    // The repository readme is the one page for nobody in particular: it
    // says what is here and sends each reader to their own.
    let repo = read("README.md");
    for artifact in ["crates/chdef/README.md", "bindings/dotnet/Chdef/README.md"] {
        assert!(
            repo.contains(artifact),
            "the repository page does not point at {artifact}"
        );
    }
}

#[test]
fn every_call_the_c_page_shows_is_one_the_header_declares() {
    // C has no doctest and CI carries no C toolchain, so the example
    // cannot be compiled here. What can be checked is the failure that
    // actually happens: a call renamed out from under the page.
    let page = read("crates/chdef-capi/README.md");
    let header = read("crates/chdef-capi/include/chdef.h");

    // Only the C blocks: the prose around them names library files, which
    // are not calls.
    let mut code = String::new();
    let mut inside = false;
    for line in page.lines() {
        match line.trim() {
            "```c" => inside = true,
            "```" if inside => inside = false,
            _ if inside => {
                code.push_str(line);
                code.push('\n');
            }
            _ => {}
        }
    }
    assert!(!code.is_empty(), "the C page shows no C");

    let mut named: Vec<String> = Vec::new();
    let mut word = String::new();
    for ch in code.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            if word.starts_with("chdef_") || word.starts_with("CHDEF_") {
                named.push(word.clone());
            }
            word.clear();
        }
    }
    named.sort();
    named.dedup();
    assert!(named.len() >= 8, "the page names only {named:?}");

    for name in named {
        assert!(
            header.contains(&format!("{name}(")) || header.contains(&format!("#define {name} ")),
            "the C page calls `{name}`, which include/chdef.h does not declare"
        );
    }
}
