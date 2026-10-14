//! Identifier pasting for `define_errors!`, on `proc_macro` alone.
//!
//! Replaces the third-party `pastey` crate for the one thing
//! `error-forge` needs from it: building constructor names such as
//! `fn [<$variant:lower>]`. For that subset it behaves like `pastey`: a
//! leading `r#` is dropped from each segment, case conversion uses
//! `str::to_lowercase` / `str::to_uppercase`, and the new identifier
//! takes the span of the `[< ... >]` bracket.

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

type Result<T> = std::result::Result<T, (Span, &'static str)>;

pub(crate) fn expand(input: TokenStream) -> TokenStream {
    match expand_stream(input) {
        Ok((output, _)) => output,
        Err((span, message)) => compile_error(span, message),
    }
}

/// Replace every `[< ... >]` in `input`, descending into groups.
/// Returns the new stream and whether anything was replaced; a group
/// whose contents are unchanged is passed through as it came in.
fn expand_stream(input: TokenStream) -> Result<(TokenStream, bool)> {
    let mut output = Vec::new();
    let mut changed = false;
    for token in input {
        match token {
            TokenTree::Group(group) if is_paste(&group) => {
                output.push(TokenTree::Ident(paste(&group)?));
                changed = true;
            }
            TokenTree::Group(group) => {
                let (stream, nested_changed) = expand_stream(group.stream())?;
                if nested_changed {
                    let mut rebuilt = Group::new(group.delimiter(), stream);
                    rebuilt.set_span(group.span());
                    output.push(TokenTree::Group(rebuilt));
                    changed = true;
                } else {
                    output.push(TokenTree::Group(group));
                }
            }
            other => output.push(other),
        }
    }
    Ok((output.into_iter().collect(), changed))
}

/// A `[...]` group whose contents start with `<` and end with `>`.
fn is_paste(group: &Group) -> bool {
    if group.delimiter() != Delimiter::Bracket {
        return false;
    }
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    matches!(
        (tokens.first(), tokens.last()),
        (Some(TokenTree::Punct(open)), Some(TokenTree::Punct(close)))
            if tokens.len() >= 2 && open.as_char() == '<' && close.as_char() == '>'
    )
}

/// Build the identifier for one `[< ... >]` group.
fn paste(group: &Group) -> Result<Ident> {
    let tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    let inner = &tokens[1..tokens.len() - 1];
    let mut name = String::new();
    let mut i = 0;
    while i < inner.len() {
        let mut segment = match &inner[i] {
            TokenTree::Ident(ident) => {
                let text = ident.to_string();
                match text.strip_prefix("r#") {
                    Some(stripped) => stripped.to_owned(),
                    None => text,
                }
            }
            other => return Err((other.span(), "expected an identifier in `[< ... >]`")),
        };
        i += 1;
        if let Some(TokenTree::Punct(colon)) = inner.get(i) {
            if colon.as_char() != ':' || colon.spacing() != Spacing::Alone {
                return Err((colon.span(), "expected `:lower` or `:upper`"));
            }
            segment = match inner.get(i + 1) {
                Some(TokenTree::Ident(modifier)) if modifier.to_string() == "lower" => {
                    segment.to_lowercase()
                }
                Some(TokenTree::Ident(modifier)) if modifier.to_string() == "upper" => {
                    segment.to_uppercase()
                }
                Some(other) => return Err((other.span(), "expected `lower` or `upper`")),
                None => return Err((colon.span(), "expected `lower` or `upper` after `:`")),
            };
            i += 2;
        }
        name.push_str(&segment);
    }
    if name.is_empty() {
        return Err((group.span(), "expected an identifier in `[< ... >]`"));
    }
    Ok(Ident::new(&name, group.span()))
}

/// `::core::compile_error! { "message" }` pointing at `span`. The path
/// is absolute so it resolves under `#![no_implicit_prelude]`.
fn compile_error(span: Span, message: &str) -> TokenStream {
    let punct = |ch, spacing| {
        let mut punct = Punct::new(ch, spacing);
        punct.set_span(span);
        TokenTree::Punct(punct)
    };
    let mut literal = proc_macro::Literal::string(message);
    literal.set_span(span);
    let mut body = Group::new(
        Delimiter::Brace,
        TokenStream::from(TokenTree::Literal(literal)),
    );
    body.set_span(span);
    [
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("core", span)),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("compile_error", span)),
        punct('!', Spacing::Alone),
        TokenTree::Group(body),
    ]
    .into_iter()
    .collect()
}
