extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
/// as its name. The string may reference any subset of the variant's
/// fields: by name for struct-like variants, by position (`{0}`, `{}`)
/// for tuple variants. Fields it does not mention are ignored.
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
/// Unsupported input (a union) produces a `compile_error!` pointing at
/// the item instead of panicking inside the compiler.
fn expand(input: &DeriveInput) -> proc_macro2::TokenStream {
    // Check if this is an enum or struct
    let is_enum = match &input.data {
        Data::Enum(_) => true,
        Data::Struct(_) => false,
        Data::Union(data) => {
            return syn::Error::new_spanned(
                data.union_token,
                "ModError cannot be derived for unions; use an enum or a struct",
            )
            .to_compile_error();
        }
    };

    // Get the error prefix from attributes
    let error_prefix = get_error_prefix(&input.attrs);

    // Generate implementation based on whether it's an enum or struct
    if is_enum {
        implement_for_enum(input, &error_prefix)
    } else {
        implement_for_struct(input, &error_prefix)
    }
}

/// Rewrite a display format so it only needs the fields it mentions.
///
/// `positional` lists the binding names in field order: the field names
/// of a struct-like variant, or `_0`, `_1`, ... for a tuple variant.
/// Positional placeholders (`{}`, `{0}`, `{0:?}`) are rewritten to those
/// names, so the generated `format!` can pass exactly the referenced
/// fields as named arguments. Passing every field used to fail with
/// "argument never used" whenever the string skipped one, including the
/// default display (the bare variant name) on variants with fields.
///
/// Returns the rewritten string and the referenced bindings in first-use
/// order, or `None` for strings this does not handle (width or precision
/// taken from an argument, malformed braces, out-of-range indices); the
/// caller then falls back to passing every field.
fn rewrite_format(fmt: &str, positional: &[String]) -> Option<(String, Vec<String>)> {
    let mut out = String::with_capacity(fmt.len());
    let mut used: Vec<String> = Vec::new();
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
                if spec.is_some_and(|spec| spec.contains('$') || spec.contains('*')) {
                    return None;
                }
                let name = if arg.is_empty() {
                    let name = positional.get(next_implicit)?.clone();
                    next_implicit += 1;
                    name
                } else if arg.bytes().all(|b| b.is_ascii_digit()) {
                    positional.get(arg.parse::<usize>().ok()?)?.clone()
                } else if is_plain_ident(arg) {
                    arg.to_string()
                } else {
                    return None;
                };
                if positional.contains(&name) && !used.contains(&name) {
                    used.push(name.clone());
                }
                out.push('{');
                out.push_str(&name);
                if let Some(spec) = spec {
                    out.push(':');
                    out.push_str(spec);
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

    Some((out, used))
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

// Extract error_prefix attribute value
fn get_error_prefix(attrs: &[syn::Attribute]) -> String {
    for attr in attrs {
        if attr.path.is_ident("error_prefix") {
            if let Some(value) = parse_string_attribute(attr) {
                return value;
            }
        }
    }
    String::new()
}

fn parse_string_attribute(attr: &syn::Attribute) -> Option<String> {
    match attr.parse_meta().ok()? {
        syn::Meta::NameValue(meta) => match meta.lit {
            syn::Lit::Str(lit) => Some(lit.value()),
            _ => None,
        },
        syn::Meta::List(meta) => match meta.nested.iter().next() {
            Some(syn::NestedMeta::Lit(syn::Lit::Str(lit))) => Some(lit.value()),
            _ => None,
        },
        syn::Meta::Path(_) => None,
    }
}

fn parse_int_attribute<T>(attr: &syn::Attribute) -> Option<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match attr.parse_meta().ok()? {
        syn::Meta::NameValue(meta) => match meta.lit {
            syn::Lit::Int(lit) => lit.base10_parse().ok(),
            _ => None,
        },
        syn::Meta::List(meta) => match meta.nested.iter().next() {
            Some(syn::NestedMeta::Lit(syn::Lit::Int(lit))) => lit.base10_parse().ok(),
            _ => None,
        },
        syn::Meta::Path(_) => None,
    }
}

fn has_flag_attribute(attr: &syn::Attribute, name: &str) -> bool {
    attr.path.is_ident(name)
}

// Implement ModError for an enum
fn implement_for_enum(input: &DeriveInput, error_prefix: &str) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let data_enum = match &input.data {
        Data::Enum(data) => data,
        _ => panic!("Expected enum"),
    };

    // Generate match arms for each variant
    let mut kind_match_arms = Vec::new();
    let mut caption_match_arms = Vec::new();
    let mut display_match_arms = Vec::new();
    let mut retryable_match_arms = Vec::new();
    let mut fatal_match_arms = Vec::new();
    let mut status_code_match_arms = Vec::new();
    let mut exit_code_match_arms = Vec::new();

    // Process each variant
    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();

        // Default values
        let mut display_format = variant_name_str.clone();
        let mut kind_name = variant_name_str.clone();
        let mut caption = format!("{}: Error", error_prefix);
        let mut retryable = false;
        let mut fatal = false;
        let mut status_code: u16 = 500;
        let mut exit_code: i32 = 1;

        // Extract attributes
        for attr in &variant.attrs {
            if attr.path.is_ident("error_display") {
                if let Some(value) = parse_string_attribute(attr) {
                    display_format = value;
                }
            } else if attr.path.is_ident("error_kind") {
                if let Some(value) = parse_string_attribute(attr) {
                    kind_name = value;
                }
            } else if attr.path.is_ident("error_caption") {
                if let Some(value) = parse_string_attribute(attr) {
                    caption = value;
                }
            } else if attr.path.is_ident("error_retryable") {
                retryable = true;
            } else if has_flag_attribute(attr, "error_fatal") {
                fatal = true;
            } else if attr.path.is_ident("error_http_status") {
                if let Some(value) = parse_int_attribute(attr) {
                    status_code = value;
                }
            } else if attr.path.is_ident("error_exit_code") {
                if let Some(value) = parse_int_attribute(attr) {
                    exit_code = value;
                }
            }
        }

        // Generate pattern matching based on the variant's fields
        match &variant.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                // Format string handled directly in match arm

                kind_match_arms.push(quote! {
                    Self::#variant_name { .. } => #kind_name
                });

                caption_match_arms.push(quote! {
                    Self::#variant_name { .. } => #caption
                });

                let positional: Vec<String> =
                    field_names.iter().map(|name| name.to_string()).collect();
                match rewrite_format(&display_format, &positional) {
                    Some((format_str, used)) => {
                        // Bind and pass only the fields the string uses.
                        let used_fields: Vec<_> = field_names
                            .iter()
                            .filter(|name| used.contains(&name.to_string()))
                            .collect();
                        display_match_arms.push(quote! {
                            Self::#variant_name { #(#used_fields,)* .. } => format!(#format_str #(, #used_fields = #used_fields)*)
                        });
                    }
                    None => {
                        display_match_arms.push(quote! {
                            Self::#variant_name { #(#field_names),* } => format!(#display_format, #(#field_names = #field_names),*)
                        });
                    }
                }

                retryable_match_arms.push(quote! {
                    Self::#variant_name { .. } => #retryable
                });

                fatal_match_arms.push(quote! {
                    Self::#variant_name { .. } => #fatal
                });

                status_code_match_arms.push(quote! {
                    Self::#variant_name { .. } => #status_code
                });

                exit_code_match_arms.push(quote! {
                    Self::#variant_name { .. } => #exit_code
                });
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_names: Vec<_> =
                    (0..field_count).map(|i| format_ident!("_{}", i)).collect();

                // Generate display format with tuple fields
                kind_match_arms.push(quote! {
                    Self::#variant_name(..) => #kind_name
                });

                caption_match_arms.push(quote! {
                    Self::#variant_name(..) => #caption
                });

                let field_pattern_list = field_names.iter().map(|name| quote! { #name, });
                let positional: Vec<String> =
                    field_names.iter().map(|name| name.to_string()).collect();
                match rewrite_format(&display_format, &positional) {
                    Some((format_str, used)) => {
                        // `_N` bindings never trigger unused-variable
                        // warnings, so every field can stay bound.
                        let used_fields: Vec<_> = field_names
                            .iter()
                            .filter(|name| used.contains(&name.to_string()))
                            .collect();
                        display_match_arms.push(quote! {
                            Self::#variant_name(#(#field_pattern_list)*) => format!(#format_str #(, #used_fields = #used_fields)*)
                        });
                    }
                    None => {
                        display_match_arms.push(quote! {
                            Self::#variant_name(#(#field_pattern_list)*) => format!(#display_format #(, #field_names)*)
                        });
                    }
                }

                retryable_match_arms.push(quote! {
                    Self::#variant_name(..) => #retryable
                });

                fatal_match_arms.push(quote! {
                    Self::#variant_name(..) => #fatal
                });

                status_code_match_arms.push(quote! {
                    Self::#variant_name(..) => #status_code
                });

                exit_code_match_arms.push(quote! {
                    Self::#variant_name(..) => #exit_code
                });
            }
            Fields::Unit => {
                // Unit variant (no fields)
                kind_match_arms.push(quote! {
                    Self::#variant_name => #kind_name
                });

                caption_match_arms.push(quote! {
                    Self::#variant_name => #caption
                });

                display_match_arms.push(quote! {
                    Self::#variant_name => #display_format.to_string()
                });

                retryable_match_arms.push(quote! {
                    Self::#variant_name => #retryable
                });

                fatal_match_arms.push(quote! {
                    Self::#variant_name => #fatal
                });

                status_code_match_arms.push(quote! {
                    Self::#variant_name => #status_code
                });

                exit_code_match_arms.push(quote! {
                    Self::#variant_name => #exit_code
                });
            }
        }
    }

    // Generate implementation
    quote! {
        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let msg = match self {
                    #(#display_match_arms,)*
                };
                write!(f, "{}", msg)
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
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                None
            }
        }
    }
}

// Implement ModError for a struct
fn implement_for_struct(input: &DeriveInput, error_prefix: &str) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let name_str = name.to_string();

    quote! {
        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}: Error", #error_prefix)
            }
        }

        impl ::error_forge::error::ForgeError for #name {
            fn kind(&self) -> &'static str {
                #name_str
            }

            fn caption(&self) -> &'static str {
                concat!(#error_prefix, ": Error")
            }
        }

        impl ::std::error::Error for #name {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                None
            }
        }
    }
}

// Note: The implementation now handles formatting directly in the match arms instead of using a helper function

#[cfg(test)]
mod tests {
    use super::{expand, rewrite_format};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn union_produces_compile_error_instead_of_panicking() {
        let input: syn::DeriveInput = syn::parse_quote! {
            union Bits { a: u32, b: f32 }
        };
        let output = expand(&input).to_string();
        assert!(output.contains("compile_error"), "{output}");
        assert!(output.contains("cannot be derived for unions"), "{output}");
    }

    #[test]
    fn rewrite_maps_positional_placeholders_to_bindings() {
        let tuple = names(&["_0", "_1", "_2"]);
        assert_eq!(
            rewrite_format("{} then {} and {0:?}", &tuple),
            Some((
                "{_0} then {_1} and {_0:?}".to_string(),
                names(&["_0", "_1"])
            ))
        );
        assert_eq!(
            rewrite_format("only {2}", &tuple),
            Some(("only {_2}".to_string(), names(&["_2"])))
        );
        assert_eq!(
            rewrite_format("NoFields", &tuple),
            Some(("NoFields".to_string(), Vec::new()))
        );
    }

    #[test]
    fn rewrite_keeps_named_and_escaped_placeholders() {
        let fields = names(&["host", "port"]);
        assert_eq!(
            rewrite_format("{{literal}} {port:>5} {CONST}", &fields),
            Some((
                "{{literal}} {port:>5} {CONST}".to_string(),
                names(&["port"])
            ))
        );
        // Positional references in a struct-like variant map to the
        // fields in declaration order, as format! did before.
        assert_eq!(
            rewrite_format("{}:{}", &fields),
            Some(("{host}:{port}".to_string(), names(&["host", "port"])))
        );
    }

    #[test]
    fn rewrite_declines_unsupported_strings() {
        let fields = names(&["a", "b"]);
        assert_eq!(rewrite_format("{:width$}", &fields), None);
        assert_eq!(rewrite_format("{:.*}", &fields), None);
        assert_eq!(rewrite_format("{5}", &fields), None);
        assert_eq!(rewrite_format("unclosed {", &fields), None);
        assert_eq!(rewrite_format("stray }", &fields), None);
    }
}
