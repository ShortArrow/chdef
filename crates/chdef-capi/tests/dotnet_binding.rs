//! The C# declarations are a mirror of this crate's `extern "C"` surface,
//! and a mis-declared field is silent memory corruption that no golden
//! vector can catch (ADR-0022). These tests prove the mirror is complete
//! and in order, without needing a .NET toolchain — the `dotnet` job that
//! runs the vectors through the binding proves it is also *correct*.

use std::path::Path;

fn rust() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap()
}

fn csharp() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bindings/dotnet/Chdef/Native.cs"),
    )
    .unwrap()
}

fn exported_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line
                .strip_prefix("pub extern \"C\" fn ")
                .or_else(|| line.strip_prefix("pub unsafe extern \"C\" fn "))?;
            Some(rest.split('(').next()?.to_string())
        })
        .collect()
}

fn exported_constants(source: &str) -> Vec<(String, String)> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub const CHDEF_")?;
            let (name, value) = rest.split_once('=')?;
            Some((
                format!("CHDEF_{}", name.split(':').next()?.trim()),
                value.trim().trim_end_matches(';').to_string(),
            ))
        })
        .collect()
}

/// The `repr(C)` structs and their fields, in declaration order.
fn exported_structs(source: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut structs = Vec::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() != "#[repr(C)]" {
            continue;
        }
        let name = loop {
            match lines.next().map(str::trim) {
                Some(l) if l.starts_with("pub struct ") => {
                    break l["pub struct ".len()..].trim_end_matches(" {").to_string()
                }
                Some(l) if l.starts_with("#[") => continue,
                _ => break String::new(),
            }
        };
        if name.is_empty() {
            continue;
        }
        let mut fields = Vec::new();
        for line in lines.by_ref() {
            let line = line.trim();
            if line == "}" {
                break;
            }
            if let Some(rest) = line.strip_prefix("pub ") {
                if let Some((field, ty)) = rest.split_once(':') {
                    fields.push((
                        field.trim().to_string(),
                        ty.trim().trim_end_matches(',').to_string(),
                    ));
                }
            }
        }
        structs.push((name, fields));
    }
    structs
}

/// `snake_case` as the C# side spells it.
fn pascal_case(field: &str) -> String {
    field
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// The C# type a Rust ABI type must be declared as. A wrong width here is
/// the defect class these tests exist for.
fn csharp_type(rust: &str) -> &'static str {
    match rust {
        "u32" => "uint",
        "u64" => "ulong",
        "i32" => "int",
        "i64" => "long",
        "f64" => "double",
        other => panic!("no C# mapping is stated for the ABI type `{other}`"),
    }
}

#[test]
fn the_binding_declares_every_exported_function() {
    let (rust, csharp) = (rust(), csharp());
    let functions = exported_functions(&rust);

    assert!(functions.len() >= 14, "found only {functions:?}");
    for name in functions {
        assert!(
            csharp.contains(&format!("partial {name}")) || csharp.contains(&format!(" {name}(")),
            "`{name}` is exported but bindings/dotnet/Chdef/Native.cs declares no such function"
        );
    }
}

#[test]
fn the_binding_declares_every_exported_constant_with_the_same_value() {
    let (rust, csharp) = (rust(), csharp());
    let constants = exported_constants(&rust);

    assert!(constants.len() >= 20, "found only {constants:?}");
    for (name, value) in constants {
        // The value must end where the declaration does, so `-6` cannot
        // match a declared `-66`. C# spells an unsigned literal with a
        // suffix Rust does not, so either spelling of the same number
        // counts.
        let declared = [format!("{name} = {value};"), format!("{name} = {value}u;")];
        assert!(
            declared.iter().any(|d| csharp.contains(d)),
            "Native.cs does not declare `{name} = {value}`; a constant that \
             differs silently sends the wrong request"
        );
    }
}

#[test]
fn every_struct_field_crosses_in_order_and_at_the_same_width() {
    let (rust, csharp) = (rust(), csharp());
    let structs = exported_structs(&rust);

    assert!(structs.len() >= 4, "found only {structs:?}");
    for (name, fields) in structs {
        let start = csharp
            .find(&format!("internal struct {name}"))
            .unwrap_or_else(|| panic!("Native.cs declares no struct `{name}`"));
        let end = csharp[start..]
            .find("\n}")
            .map(|i| start + i)
            .unwrap_or(csharp.len());
        let body = &csharp[start..end];

        // Sequential layout is what makes the field order meaningful.
        assert!(
            csharp[..start].contains("[StructLayout(LayoutKind.Sequential)]"),
            "`{name}` is not declared with sequential layout"
        );

        let declared: Vec<&str> = body
            .lines()
            .filter_map(|line| line.trim().strip_prefix("public "))
            .map(|rest| rest.trim_end_matches(';'))
            .collect();

        assert_eq!(
            declared.len(),
            fields.len(),
            "`{name}` has {} fields in Rust and {} in Native.cs: {declared:?}",
            fields.len(),
            declared.len()
        );
        for (index, (field, ty)) in fields.iter().enumerate() {
            let expected = format!("{} {}", csharp_type(ty), pascal_case(field));
            assert_eq!(
                declared[index], expected,
                "`{name}` field {index} is `{field}: {ty}` in Rust; Native.cs must \
                 declare it as `{expected}`"
            );
        }
    }
}

#[test]
fn the_binding_names_the_library_the_crate_builds() {
    let csharp = csharp();

    assert!(
        csharp.contains("internal const string Library = \"chdef_capi\";"),
        "Native.cs must load the library `chdef_capi`, the `[lib] name` of this crate"
    );
}
