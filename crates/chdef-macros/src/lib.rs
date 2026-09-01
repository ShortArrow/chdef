//! A definition file expanded into a constant table in place, where
//! `chdef-gen` would have written a file (`docs/spec/embedded.md` §3).
//!
//! `layout!("ch.csv")` reads the CSV at compile time on the host and emits
//! exactly the items `chdef_gen::rust_source` writes, wrapped in a module
//! and re-exported. The definition is a build input: the expansion embeds
//! its bytes with `include_bytes!`, so an edit to the CSV rebuilds the
//! crate and a stale table cannot survive one.
//!
//! The items name `chdef_core::…`, which the invoking crate supplies; this
//! crate runs on the host and never reaches the target (ADR-0035).

use chdef::ColumnVocabulary;
use chdef_core::Endian;
use proc_macro::{TokenStream, TokenTree};

/// What an invocation may say, for the message that says it did not.
const GRAMMAR: &str = "the invocation is `layout!(\"ch.csv\")` followed by any of \
                       `, bf = \"bf.csv\"`, `, endian = little`, `, endian = big` and `, japanese`";

/// A CH CSV, and an optional BF CSV, as the constant table they describe.
///
/// ```ignore
/// chdef_macros::layout!("ch.csv");
/// chdef_macros::layout!("ch.csv", bf = "bf.csv", endian = big, japanese);
/// ```
///
/// Paths are relative to the invoking crate's `CARGO_MANIFEST_DIR`, the one
/// directory a procedural macro can know. The options come in any order,
/// each at most once, with a trailing comma allowed.
///
/// The expansion declares `LAYOUT` and one `CH_…` constant per named
/// channel. A definition with any Issue is refused, and the refusal is a
/// `compile_error!` carrying the findings `chdef-gen` would have printed.
#[proc_macro]
pub fn layout(input: TokenStream) -> TokenStream {
    match expansion(input) {
        Ok(items) => items,
        Err(message) => compile_error(&message),
    }
}

/// The items the invocation expands to, or the one message saying why it
/// expands to none.
fn expansion(input: TokenStream) -> Result<TokenStream, String> {
    let invocation = invocation(input)?;
    let directory = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        "chdef-macros: CARGO_MANIFEST_DIR is not set, so a relative path has no root".to_string()
    })?;

    let ch_path = joined(&directory, &invocation.ch);
    let ch = read(&ch_path)?;
    let bf_path = invocation.bf.as_deref().map(|bf| joined(&directory, bf));
    let bf = match &bf_path {
        Some(path) => read(path)?,
        None => Vec::new(),
    };

    let vocabulary = if invocation.japanese {
        ColumnVocabulary::japanese()
    } else {
        ColumnVocabulary::new()
    };
    let model = chdef_gen::model(&ch, &bf, invocation.endian, &vocabulary)
        .map_err(|refusal| format!("chdef-macros: the definition was refused\n{refusal}"))?;

    let origin = match &invocation.bf {
        Some(bf) => format!("{} + {}", invocation.ch, bf),
        None => invocation.ch.clone(),
    };

    let mut source = String::from("mod chdef_layout {\n");
    source.push_str(&format!("const _: &[u8] = include_bytes!({ch_path:?});\n"));
    if let Some(path) = &bf_path {
        source.push_str(&format!("const _: &[u8] = include_bytes!({path:?});\n"));
    }
    source.push_str(&chdef_gen::rust_source(&model, &origin));
    source.push_str("}\npub use chdef_layout::*;\n");

    source
        .parse()
        .map_err(|error| format!("chdef-macros: the table is not the Rust it should be: {error}"))
}

/// A message as the only item the invocation expands to.
fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .unwrap_or_default()
}

/// A path as the invocation wrote it, under the invoking crate's directory.
fn joined(directory: &str, path: &str) -> String {
    std::path::Path::new(directory)
        .join(path)
        .display()
        .to_string()
}

/// A definition file's bytes, or the line saying it could not be read.
fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("chdef-macros: {path} could not be read: {error}"))
}

/// What the arguments asked for.
struct Invocation {
    ch: String,
    bf: Option<String>,
    endian: Endian,
    japanese: bool,
}

/// The arguments read by walking the tokens: a path, then options
/// introduced by commas, in any order and each at most once.
fn invocation(input: TokenStream) -> Result<Invocation, String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut at = 0;
    let mut invocation = Invocation {
        ch: text(&tokens, &mut at)?,
        bf: None,
        endian: Endian::Little,
        japanese: false,
    };
    let mut endian_given = false;

    while at < tokens.len() {
        punctuation(&tokens, &mut at, ',')?;
        if at == tokens.len() {
            break;
        }
        let option = name(&tokens, &mut at)?;
        match option.as_str() {
            "bf" => {
                if invocation.bf.is_some() {
                    return Err(given_twice("bf"));
                }
                punctuation(&tokens, &mut at, '=')?;
                invocation.bf = Some(text(&tokens, &mut at)?);
            }
            "endian" => {
                if endian_given {
                    return Err(given_twice("endian"));
                }
                endian_given = true;
                punctuation(&tokens, &mut at, '=')?;
                let word = name(&tokens, &mut at)?;
                invocation.endian = match word.as_str() {
                    "little" => Endian::Little,
                    "big" => Endian::Big,
                    _ => {
                        return Err(format!(
                            "chdef-macros: `endian` is `little` or `big`, not `{word}`"
                        ))
                    }
                };
            }
            "japanese" => {
                if invocation.japanese {
                    return Err(given_twice("japanese"));
                }
                invocation.japanese = true;
            }
            _ => {
                return Err(format!(
                    "chdef-macros: `{option}` is not an option of this macro; {GRAMMAR}"
                ))
            }
        }
    }
    Ok(invocation)
}

fn given_twice(option: &str) -> String {
    format!("chdef-macros: `{option}` is given twice; each option is given at most once")
}

/// The string literal at `at`, as the text it stands for.
fn text(tokens: &[TokenTree], at: &mut usize) -> Result<String, String> {
    let Some(TokenTree::Literal(literal)) = tokens.get(*at) else {
        return Err(format!(
            "chdef-macros: a path in double quotes was expected; {GRAMMAR}"
        ));
    };
    let text = unquoted(&literal.to_string())?;
    *at += 1;
    Ok(text)
}

/// The bare identifier at `at`.
fn name(tokens: &[TokenTree], at: &mut usize) -> Result<String, String> {
    let Some(TokenTree::Ident(identifier)) = tokens.get(*at) else {
        return Err(format!(
            "chdef-macros: an option name was expected; {GRAMMAR}"
        ));
    };
    *at += 1;
    Ok(identifier.to_string())
}

/// The one punctuation character at `at`.
fn punctuation(tokens: &[TokenTree], at: &mut usize, wanted: char) -> Result<(), String> {
    match tokens.get(*at) {
        Some(TokenTree::Punct(punct)) if punct.as_char() == wanted => {
            *at += 1;
            Ok(())
        }
        _ => Err(format!("chdef-macros: `{wanted}` was expected; {GRAMMAR}")),
    }
}

/// A string literal as written, as the string it means. A raw string is
/// taken as it stands; the escapes a path can hold are undone.
fn unquoted(literal: &str) -> Result<String, String> {
    if let Some(body) = raw_body(literal) {
        return Ok(body.to_string());
    }
    let Some(body) = literal
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return Err(format!(
            "chdef-macros: `{literal}` is not a path in double quotes"
        ));
    };

    let mut out = String::new();
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some(other) => {
                return Err(format!(
                    "chdef-macros: `\\{other}` in `{literal}` is not an escape this macro reads"
                ))
            }
            None => return Err(format!("chdef-macros: `{literal}` ends in a backslash")),
        }
    }
    Ok(out)
}

/// The body of `r"…"` or `r#"…"#`, or `None` where the literal is not raw.
fn raw_body(literal: &str) -> Option<&str> {
    let rest = literal.strip_prefix('r')?;
    let hashes = "#".repeat(rest.len() - rest.trim_start_matches('#').len());
    rest.strip_prefix(hashes.as_str())?
        .strip_prefix('"')?
        .strip_suffix(hashes.as_str())?
        .strip_suffix('"')
}
