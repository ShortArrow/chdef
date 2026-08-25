//! The NuGet front page and the tests that back it say the same thing.
//!
//! C# has no doctest: an XML `<example>` block is a string, and nothing
//! compiles it. So the examples live in `Chdef.Tests/ReadmeTests.cs`,
//! where they are compiled and run, and this test fails the build if the
//! page and that file drift apart — the discipline `header.rs` and
//! `dotnet_binding.rs` already use for the declarations.
//!
//! Which file each package *carries* is `front_pages.rs`.

use std::path::Path;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every fenced `csharp` block of the readme, in order.
fn readme_examples() -> Vec<String> {
    let readme = read("bindings/dotnet/Chdef/README.md");
    let mut examples = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in readme.lines() {
        match (line.trim(), current.as_mut()) {
            ("```csharp", None) => current = Some(Vec::new()),
            ("```", Some(_)) => examples.push(current.take().unwrap().join("\n")),
            (_, Some(block)) => block.push(line),
            _ => {}
        }
    }
    assert!(current.is_none(), "an unterminated code fence");
    examples
}

/// The body of each `[Fact]` in the mirror file, in order, with the
/// method's indentation removed.
fn test_bodies() -> Vec<String> {
    let source = read("bindings/dotnet/Chdef.Tests/ReadmeTests.cs");
    let lines: Vec<&str> = source.lines().collect();
    let mut bodies = Vec::new();

    let mut index = 0;
    while index < lines.len() {
        if lines[index].trim() != "[Fact]" {
            index += 1;
            continue;
        }
        let open = lines[index..]
            .iter()
            .position(|l| l.trim() == "{")
            .map(|offset| index + offset)
            .expect("a method body");
        let close = lines[open..]
            .iter()
            .position(|l| l.trim_end() == "    }")
            .map(|offset| open + offset)
            .expect("a closing brace");

        let body: Vec<String> = lines[open + 1..close]
            .iter()
            .map(|line| line.strip_prefix("        ").unwrap_or(line).to_string())
            .collect();
        bodies.push(body.join("\n").trim_end().to_string());
        index = close;
    }
    bodies
}

#[test]
fn every_example_on_the_page_is_a_test_that_runs() {
    let examples = readme_examples();
    let bodies = test_bodies();

    assert!(
        examples.len() >= 4,
        "found only {} examples",
        examples.len()
    );
    assert_eq!(
        examples.len(),
        bodies.len(),
        "the page has {} examples and the mirror {} tests",
        examples.len(),
        bodies.len()
    );

    for (index, (example, body)) in examples.iter().zip(&bodies).enumerate() {
        assert_eq!(
            example.trim(),
            body.trim(),
            "example {index} of bindings/dotnet/Chdef/README.md is not what \
             Chdef.Tests/ReadmeTests.cs runs"
        );
    }
}
