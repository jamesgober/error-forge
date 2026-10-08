extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{ParseStream, Parser};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Lit, Token};

/// Derive macro for ModError
///
/// This macro automatically implements the ForgeError trait and common
/// error handling functionality for a struct or enum, allowing for
/// "lazy mode" error creation with minimal boilerplate.
///
/// # Example
///
/// When the macro is used in your application where error-forge is a dependency:
///
/// ```ignore
/// use error_forge::ModError;
///
/// #[derive(Debug, ModError)]
/// #[error_prefix("Database")]
/// pub enum DbError {
///     #[error_display("Connection to {0} failed")]
///     ConnectionFailed(String),
///
///     #[error_display("Query execution failed: {reason}")]
///     QueryFailed { reason: String },
///
///     #[error_display("Transaction error")]
///     #[error_http_status(400)]
///     TransactionError,
/// }
/// ```
///
/// `#[error_display("...")]` is optional; without it a variant displays
/// as its name. The string is a `format!` string and may reference any
/// subset of the variant's fields: by name for struct-like variants
/// (a raw identifier such as `r#type` is written `{type}`), by position
/// (`{0}`, `{}`) for tuple variants. Fields it does not mention are
/// ignored, `{{` and `}}` produce literal braces, and a width or
/// precision given where the error is formatted (`{:>20}`) pads or
/// truncates the whole message.
///
/// # Attributes
///
/// On the enum (or struct):
///
/// - `#[error_prefix("...")]`: caption prefix, giving `"<prefix>: Error"`.
///
/// On each enum variant:
///
/// - `#[error_display("...")]`, `#[error_kind("...")]`,
///   `#[error_caption("...")]`: string literals.
/// - `#[error_retryable]`, `#[error_fatal]`: flags. A bare flag means
///   `true`; an explicit value (`#[error_retryable(false)]` or
///   `#[error_retryable = false]`) is honoured.
/// - `#[error_http_status(404)]`: an integer that fits in `u16`.
/// - `#[error_exit_code(2)]`: an integer that fits in `i32`.
///
/// Each attribute accepts both the list form (`#[name(value)]`) and the
/// name-value form (`#[name = value]`). A value of the wrong type or out
/// of range is a compile error.
///
/// # Structs
///
/// On a struct only `#[error_prefix]` is read. The struct displays as
/// `"<prefix>: Error"`, its `kind()` is the struct name, and the other
/// metadata methods use the `ForgeError` defaults. The variant
/// attributes listed above (`#[error_display]`, `#[error_retryable]`,
/// and so on) are currently ignored when placed on a struct.
///
/// Note: This is a procedural macro that is re-exported by the `error-forge` crate.
/// When using in your application, import it from the main crate with `use error_forge::ModError;`.
#[proc_macro_derive(
    ModError,
    attributes(
        error_prefix,
        error_display,
        error_kind,
        error_caption,
        error_retryable,
        error_http_status,
        error_exit_code,
        error_fatal
    )
)]
pub fn derive_mod_error(input: TokenStream) -> TokenStream {
    // Parse the input
    let input = parse_macro_input!(input as DeriveInput);

    // Return the generated implementation
    TokenStream::from(expand(&input))
}

/// Expand `#[derive(ModError)]` for a parsed item.
///
/// Unsupported input (a union) and malformed attributes produce a
/// `compile_error!` pointing at the offending tokens instead of
/// panicking inside the compiler or falling back to defaults.
fn expand(input: &DeriveInput) -> TokenStream2 {
    let result = match &input.data {
        Data::Enum(_) => {
            get_error_prefix(&input.attrs).and_then(|prefix| implement_for_enum(input, &prefix))
        }
        Data::Struct(_) => {
            get_error_prefix(&input.attrs).map(|prefix| implement_for_struct(input, &prefix))
        }
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "ModError cannot be derived for unions; use an enum or a struct",
        )),
    };
    result.unwrap_or_else(|error| error.to_compile_error())
}

/// Rewrite a display format so it only needs the fields it mentions.
///
/// `fields` lists the variant's fields in declaration order: their
/// names (without any `r#` prefix) for a struct-like variant, `None` for
/// a tuple variant. Every reference to a field, whether positional
/// (`{}`, `{0}`), by name (`{host}`, `{r#type}`) or as a width or
/// precision argument (`{:width$}`), is rewritten to an explicit
/// position in the argument list the caller passes. Names that are not
/// fields (`{CONST}`) are left for `format_args!` to capture.
///
/// Returns the rewritten string and the referenced field indices in
/// argument order, or `None` for strings this does not handle (`.*`
/// precision, malformed braces, out-of-range indices); the caller then
/// passes every field and lets `format_args!` report any error.
fn rewrite_format(fmt: &str, fields: &[Option<String>]) -> Option<(String, Vec<usize>)> {
    fn slot(args: &mut Vec<usize>, field: usize) -> usize {
        match args.iter().position(|&f| f == field) {
            Some(index) => index,
            None => {
                args.push(field);
                args.len() - 1
            }
        }
    }
    fn named_field(fields: &[Option<String>], name: &str) -> Option<usize> {
        let name = name.strip_prefix("r#").unwrap_or(name);
        fields.iter().position(|f| f.as_deref() == Some(name))
    }

    let mut out = String::with_capacity(fmt.len() + 8);
    let mut args: Vec<usize> = Vec::new();
    let mut next_implicit = 0usize;
    let mut chars = fmt.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push_str("{{");
            }
            '{' => {
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some('{') | None => return None,
                        Some(ch) => inner.push(ch),
                    }
                }
                let (arg, spec) = match inner.find(':') {
                    Some(i) => (&inner[..i], Some(&inner[i + 1..])),
                    None => (inner.as_str(), None),
                };

                out.push('{');
                if arg.is_empty() {
                    if next_implicit >= fields.len() {
                        return None;
                    }
                    out.push_str(&slot(&mut args, next_implicit).to_string());
                    next_implicit += 1;
                } else if arg.bytes().all(|b| b.is_ascii_digit()) {
                    let index = arg.parse::<usize>().ok()?;
                    if index >= fields.len() {
                        return None;
                    }
                    out.push_str(&slot(&mut args, index).to_string());
                } else if is_plain_ident(arg.strip_prefix("r#").unwrap_or(arg)) {
                    match named_field(fields, arg) {
                        Some(index) => out.push_str(&slot(&mut args, index).to_string()),
                        None => out.push_str(arg),
                    }
                } else {
                    return None;
                }

                if let Some(spec) = spec {
                    if spec.contains('*') {
                        return None;
                    }
                    out.push(':');
                    // Map `name$` / `N$` width and precision arguments.
                    let mut word = String::new();
                    for ch in spec.chars() {
                        if ch == '_' || ch.is_alphanumeric() {
                            word.push(ch);
                            continue;
                        }
                        if ch == '$' && !word.is_empty() {
                            let field = if word.bytes().all(|b| b.is_ascii_digit()) {
                                let index = word.parse::<usize>().ok()?;
                                if index >= fields.len() {
                                    return None;
                                }
                                Some(index)
                            } else {
                                named_field(fields, &word)
                            };
                            match field {
                                Some(index) => out.push_str(&slot(&mut args, index).to_string()),
                                None => out.push_str(&word),
                            }
                        } else {
                            out.push_str(&word);
                        }
                        word.clear();
                        out.push(ch);
                    }
                    out.push_str(&word);
                }
                out.push('}');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push_str("}}");
            }
            '}' => return None,
            other => out.push(other),
        }
    }

    Some((out, args))
}

fn is_plain_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_alphabetic() => {
            chars.all(|c| c == '_' || c.is_alphanumeric())
        }
        _ => false,
    }
}

/// The value written after an attribute's path.
enum AttrValue {
    /// `#[name]`
    Flag,
    /// `#[name(value)]` or `#[name = value]`, with an optional leading
    /// minus sign for negative integers.
    Lit(Option<Token![-]>, Lit),
}

fn parse_attr_value(input: ParseStream) -> syn::Result<AttrValue> {
    fn literal(input: ParseStream) -> syn::Result<AttrValue> {
        let minus: Option<Token![-]> = input.parse()?;
        let lit: Lit = input.parse()?;
        Ok(AttrValue::Lit(minus, lit))
    }

    if input.is_empty() {
        return Ok(AttrValue::Flag);
    }
    let value = if input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in input);
        let value = literal(&content)?;
        if !content.is_empty() {
            return Err(content.error("expected a single literal value"));
        }
        value
    } else if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        literal(input)?
    } else {
        return Err(input.error("expected `(value)` or `= value`"));
    };
    if !input.is_empty() {
        return Err(input.error("unexpected tokens after the attribute value"));
    }
    Ok(value)
}

fn attr_value(attr: &syn::Attribute) -> syn::Result<AttrValue> {
    parse_attr_value.parse2(attr.tokens.clone())
}

fn attr_name(attr: &syn::Attribute) -> String {
    attr.path
        .get_ident()
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn parse_string_attribute(attr: &syn::Attribute) -> syn::Result<String> {
    let message = || format!("`{}` expects a string literal", attr_name(attr));
    match attr_value(attr)? {
        AttrValue::Lit(None, Lit::Str(lit)) => Ok(lit.value()),
        AttrValue::Lit(Some(minus), _) => Err(syn::Error::new_spanned(minus, message())),
        AttrValue::Lit(None, lit) => Err(syn::Error::new_spanned(lit, message())),
        AttrValue::Flag => Err(syn::Error::new_spanned(
            attr,
            format!("{}: `#[{}(\"...\")]`", message(), attr_name(attr)),
        )),
    }
}

fn parse_flag_attribute(attr: &syn::Attribute) -> syn::Result<bool> {
    let message = || format!("`{}` expects `true` or `false`", attr_name(attr));
    match attr_value(attr)? {
        AttrValue::Flag => Ok(true),
        AttrValue::Lit(None, Lit::Bool(lit)) => Ok(lit.value),
        AttrValue::Lit(Some(minus), _) => Err(syn::Error::new_spanned(minus, message())),
        AttrValue::Lit(None, lit) => Err(syn::Error::new_spanned(lit, message())),
    }
}

fn parse_int_attribute<T>(attr: &syn::Attribute) -> syn::Result<T>
where
    T: TryFrom<i128>,
{
    let name = attr_name(attr);
    let range_error = |tokens: TokenStream2| {
        syn::Error::new_spanned(
            tokens,
            format!(
                "`{name}` expects an integer in {}",
                std::any::type_name::<T>()
            ),
        )
    };
    match attr_value(attr)? {
        AttrValue::Lit(minus, Lit::Int(lit)) => {
            let tokens = quote!(#minus #lit);
            let magnitude: i128 = lit
                .base10_parse()
                .map_err(|_| range_error(tokens.clone()))?;
            let value = if minus.is_some() {
                -magnitude
            } else {
                magnitude
            };
            T::try_from(value).map_err(|_| range_error(tokens))
        }
        AttrValue::Lit(_, lit) => Err(syn::Error::new_spanned(
            lit,
            format!("`{name}` expects an integer literal"),
        )),
        AttrValue::Flag => Err(syn::Error::new_spanned(
            attr,
            format!("`{name}` expects an integer: `#[{name}(...)]`"),
        )),
    }
}

/// Merge `error` into the running error so every bad attribute is
/// reported, not only the first.
fn push_error(errors: &mut Option<syn::Error>, error: syn::Error) {
    match errors {
        Some(existing) => existing.combine(error),
        None => *errors = Some(error),
    }
}

// Extract error_prefix attribute value
fn get_error_prefix(attrs: &[syn::Attribute]) -> syn::Result<String> {
    let mut prefix = String::new();
    for attr in attrs {
        if attr.path.is_ident("error_prefix") {
            prefix = parse_string_attribute(attr)?;
        }
    }
    Ok(prefix)
}

/// Metadata read from one variant's attributes.
struct VariantAttrs {
    display: String,
    kind: String,
    caption: String,
    retryable: bool,
    fatal: bool,
    status_code: u16,
    exit_code: i32,
}

fn variant_attrs(variant: &syn::Variant, error_prefix: &str) -> syn::Result<VariantAttrs> {
    let name = variant.ident.to_string();
    let mut attrs = VariantAttrs {
        display: name.clone(),
        kind: name,
        caption: format!("{}: Error", error_prefix),
        retryable: false,
        fatal: false,
        status_code: 500,
        exit_code: 1,
    };
    let mut errors = None;
    for attr in &variant.attrs {
        let result = if attr.path.is_ident("error_display") {
            parse_string_attribute(attr).map(|v| attrs.display = v)
        } else if attr.path.is_ident("error_kind") {
            parse_string_attribute(attr).map(|v| attrs.kind = v)
        } else if attr.path.is_ident("error_caption") {
            parse_string_attribute(attr).map(|v| attrs.caption = v)
        } else if attr.path.is_ident("error_retryable") {
            parse_flag_attribute(attr).map(|v| attrs.retryable = v)
        } else if attr.path.is_ident("error_fatal") {
            parse_flag_attribute(attr).map(|v| attrs.fatal = v)
        } else if attr.path.is_ident("error_http_status") {
            parse_int_attribute(attr).map(|v| attrs.status_code = v)
        } else if attr.path.is_ident("error_exit_code") {
            parse_int_attribute(attr).map(|v| attrs.exit_code = v)
        } else {
            Ok(())
        };
        if let Err(error) = result {
            push_error(&mut errors, error);
        }
    }
    match errors {
        Some(error) => Err(error),
        None => Ok(attrs),
    }
}

/// Build the `Display` match arm for one variant. The arm writes into
/// `writer`, a `&mut dyn core::fmt::Write`.
fn display_arm(variant: &syn::Variant, display: &str, writer: &Ident) -> TokenStream2 {
    let variant_name = &variant.ident;
    // Bindings use mixed-site hygiene so a display string cannot capture
    // them by accident and they cannot collide with user items.
    let binding = |index: usize| Ident::new(&format!("__ef_field_{index}"), Span::mixed_site());

    match &variant.fields {
        Fields::Named(fields) => {
            let idents: Vec<&Ident> = fields
                .named
                .iter()
                .map(|f| f.ident.as_ref().expect("named field"))
                .collect();
            let names: Vec<Option<String>> = idents
                .iter()
                .map(|ident| Some(ident.unraw().to_string()))
                .collect();
            match rewrite_format(display, &names) {
                Some((format_str, args)) => {
                    let pattern = args.iter().map(|&i| {
                        let field = idents[i];
                        let bind = binding(i);
                        quote! { #field: #bind }
                    });
                    let values = args.iter().map(|&i| binding(i));
                    quote! {
                        Self::#variant_name { #(#pattern,)* .. } =>
                            ::core::write!(#writer, #format_str #(, #values)*)
                    }
                }
                None => {
                    let binds: Vec<Ident> = (0..idents.len()).map(binding).collect();
                    quote! {
                        Self::#variant_name { #(#idents: #binds),* } =>
                            ::core::write!(#writer, #display, #(#idents = #binds),*)
                    }
                }
            }
        }
        Fields::Unnamed(fields) => {
            let count = fields.unnamed.len();
            let names: Vec<Option<String>> = vec![None; count];
            match rewrite_format(display, &names) {
                Some((format_str, args)) => {
                    let pattern = (0..count).map(|i| {
                        if args.contains(&i) {
                            let bind = binding(i);
                            quote! { #bind }
                        } else {
                            quote! { _ }
                        }
                    });
                    let values = args.iter().map(|&i| binding(i));
                    quote! {
                        Self::#variant_name(#(#pattern),*) =>
                            ::core::write!(#writer, #format_str #(, #values)*)
                    }
                }
                None => {
                    let binds: Vec<Ident> = (0..count).map(binding).collect();
                    quote! {
                        Self::#variant_name(#(#binds),*) =>
                            ::core::write!(#writer, #display #(, #binds)*)
                    }
                }
            }
        }
        Fields::Unit => {
            let format_str = rewrite_format(display, &[])
                .map(|(format_str, _)| format_str)
                .unwrap_or_else(|| display.to_string());
            quote! {
                Self::#variant_name => ::core::write!(#writer, #format_str)
            }
        }
    }
}

// Implement ModError for an enum
fn implement_for_enum(input: &DeriveInput, error_prefix: &str) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let data_enum = match &input.data {
        Data::Enum(data) => data,
        _ => unreachable!("implement_for_enum is only called for enums"),
    };

    let writer = Ident::new("__ef_writer", Span::mixed_site());
    let formatter = Ident::new("__ef_formatter", Span::mixed_site());
    let write_fn = Ident::new("__ef_write", Span::mixed_site());
    let buffer = Ident::new("__ef_buffer", Span::mixed_site());

    // Generate match arms for each variant
    let mut kind_match_arms = Vec::new();
    let mut caption_match_arms = Vec::new();
    let mut display_match_arms = Vec::new();
    let mut retryable_match_arms = Vec::new();
    let mut fatal_match_arms = Vec::new();
    let mut status_code_match_arms = Vec::new();
    let mut exit_code_match_arms = Vec::new();
    let mut errors = None;

    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let attrs = match variant_attrs(variant, error_prefix) {
            Ok(attrs) => attrs,
            Err(error) => {
                push_error(&mut errors, error);
                continue;
            }
        };
        let VariantAttrs {
            display,
            kind,
            caption,
            retryable,
            fatal,
            status_code,
            exit_code,
        } = attrs;

        let pattern = match &variant.fields {
            Fields::Named(_) => quote! { Self::#variant_name { .. } },
            Fields::Unnamed(_) => quote! { Self::#variant_name(..) },
            Fields::Unit => quote! { Self::#variant_name },
        };

        kind_match_arms.push(quote! { #pattern => #kind });
        caption_match_arms.push(quote! { #pattern => #caption });
        retryable_match_arms.push(quote! { #pattern => #retryable });
        fatal_match_arms.push(quote! { #pattern => #fatal });
        status_code_match_arms.push(quote! { #pattern => #status_code });
        exit_code_match_arms.push(quote! { #pattern => #exit_code });
        display_match_arms.push(display_arm(variant, &display, &writer));
    }

    if let Some(error) = errors {
        return Err(error);
    }

    // Generate implementation
    Ok(quote! {
        impl ::core::fmt::Display for #name {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let #write_fn = |#writer: &mut dyn ::core::fmt::Write| -> ::core::fmt::Result {
                    match self {
                        #(#display_match_arms,)*
                    }
                };
                if #formatter.width().is_none() && #formatter.precision().is_none() {
                    #write_fn(#formatter)
                } else {
                    // Width and precision apply to the whole message, so
                    // it has to be rendered before it can be padded.
                    let mut #buffer = ::std::string::String::new();
                    #write_fn(&mut #buffer)?;
                    #formatter.pad(&#buffer)
                }
            }
        }

        impl ::error_forge::error::ForgeError for #name {
            fn kind(&self) -> &'static str {
                match self {
                    #(#kind_match_arms,)*
                }
            }

            fn caption(&self) -> &'static str {
                match self {
                    #(#caption_match_arms,)*
                }
            }

            fn is_retryable(&self) -> bool {
                match self {
                    #(#retryable_match_arms,)*
                }
            }

            fn is_fatal(&self) -> bool {
                match self {
                    #(#fatal_match_arms,)*
                }
            }

            fn status_code(&self) -> u16 {
                match self {
                    #(#status_code_match_arms,)*
                }
            }

            fn exit_code(&self) -> i32 {
                match self {
                    #(#exit_code_match_arms,)*
                }
            }
        }

        impl ::std::error::Error for #name {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                ::core::option::Option::None
            }
        }
    })
}

// Implement ModError for a struct
fn implement_for_struct(input: &DeriveInput, error_prefix: &str) -> TokenStream2 {
    let name = &input.ident;
    let name_str = name.to_string();
    let formatter = Ident::new("__ef_formatter", Span::mixed_site());

    quote! {
        impl ::core::fmt::Display for #name {
            fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #formatter.pad(::core::concat!(#error_prefix, ": Error"))
            }
        }

        impl ::error_forge::error::ForgeError for #name {
            fn kind(&self) -> &'static str {
                #name_str
            }

            fn caption(&self) -> &'static str {
                ::core::concat!(#error_prefix, ": Error")
            }
        }

        impl ::std::error::Error for #name {
            fn source(&self) -> ::core::option::Option<&(dyn ::std::error::Error + 'static)> {
                ::core::option::Option::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{expand, rewrite_format};

    fn named(list: &[&str]) -> Vec<Option<String>> {
        list.iter().map(|s| Some(s.to_string())).collect()
    }

    fn tuple(count: usize) -> Vec<Option<String>> {
        vec![None; count]
    }

    fn expand_str(input: syn::DeriveInput) -> String {
        expand(&input).to_string()
    }

    #[test]
    fn union_produces_compile_error_instead_of_panicking() {
        let output = expand_str(syn::parse_quote! {
            union Bits { a: u32, b: f32 }
        });
        assert!(output.contains("compile_error"), "{output}");
        assert!(output.contains("cannot be derived for unions"), "{output}");
    }

    #[test]
    fn rewrite_maps_positional_placeholders_to_arguments() {
        assert_eq!(
            rewrite_format("{} then {} and {0:?}", &tuple(3)),
            Some(("{0} then {1} and {0:?}".to_string(), vec![0, 1]))
        );
        assert_eq!(
            rewrite_format("only {2}", &tuple(3)),
            Some(("only {0}".to_string(), vec![2]))
        );
        assert_eq!(
            rewrite_format("NoFields", &tuple(3)),
            Some(("NoFields".to_string(), Vec::new()))
        );
    }

    #[test]
    fn rewrite_keeps_escapes_and_non_field_names() {
        let fields = named(&["host", "port"]);
        assert_eq!(
            rewrite_format("{{literal}} {port:>5} {CONST}", &fields),
            Some(("{{literal}} {0:>5} {CONST}".to_string(), vec![1]))
        );
        // Positional references in a struct-like variant map to the
        // fields in declaration order, as format! did before.
        assert_eq!(
            rewrite_format("{}:{}", &fields),
            Some(("{0}:{1}".to_string(), vec![0, 1]))
        );
    }

    #[test]
    fn rewrite_maps_raw_identifiers_and_count_arguments() {
        let fields = named(&["type", "width"]);
        assert_eq!(
            rewrite_format("{type} {r#type} {type:>width$} {:.1$}", &fields),
            Some(("{0} {0} {0:>1$} {0:.1$}".to_string(), vec![0, 1]))
        );
        assert_eq!(
            rewrite_format("{type:>W$}", &fields),
            Some(("{0:>W$}".to_string(), vec![0]))
        );
    }

    #[test]
    fn rewrite_declines_unsupported_strings() {
        let fields = named(&["a", "b"]);
        assert_eq!(rewrite_format("{:.*}", &fields), None);
        assert_eq!(rewrite_format("{5}", &fields), None);
        assert_eq!(rewrite_format("{} {} {}", &fields), None);
        assert_eq!(rewrite_format("unclosed {", &fields), None);
        assert_eq!(rewrite_format("stray }", &fields), None);
    }

    #[test]
    fn malformed_attribute_values_are_compile_errors() {
        let cases: [(syn::DeriveInput, &str); 6] = [
            (
                syn::parse_quote! { enum E { #[error_http_status("404")] A } },
                "`error_http_status` expects an integer literal",
            ),
            (
                syn::parse_quote! { enum E { #[error_exit_code(99999999999)] A } },
                "`error_exit_code` expects an integer in i32",
            ),
            (
                syn::parse_quote! { enum E { #[error_http_status(-1)] A } },
                "`error_http_status` expects an integer in u16",
            ),
            (
                syn::parse_quote! { enum E { #[error_retryable("yes")] A } },
                "`error_retryable` expects `true` or `false`",
            ),
            (
                syn::parse_quote! { enum E { #[error_display(42)] A } },
                "`error_display` expects a string literal",
            ),
            (
                syn::parse_quote! { #[error_prefix] enum E { A } },
                "`error_prefix` expects a string literal",
            ),
        ];
        for (input, message) in cases {
            let output = expand_str(input);
            assert!(output.contains("compile_error"), "{output}");
            assert!(output.contains(message), "{message} not in {output}");
        }
    }

    #[test]
    fn explicit_flag_values_are_honoured() {
        let output = expand_str(syn::parse_quote! {
            enum E {
                #[error_retryable(false)]
                #[error_fatal = false]
                A,
                #[error_retryable = true]
                #[error_fatal(true)]
                B,
                #[error_exit_code(-2)]
                C,
            }
        });
        assert!(!output.contains("compile_error"), "{output}");
        let retryable = output
            .split("fn is_retryable")
            .nth(1)
            .and_then(|s| s.split("fn is_fatal").next())
            .unwrap();
        assert!(retryable.contains("Self :: A => false"), "{retryable}");
        assert!(retryable.contains("Self :: B => true"), "{retryable}");
        let exit = output.split("fn exit_code").nth(1).unwrap();
        assert!(
            exit.contains("Self :: C => - 2i32") || exit.contains("Self :: C => -2i32"),
            "{exit}"
        );
    }
}
