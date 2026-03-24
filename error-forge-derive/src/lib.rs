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

    // Check if this is an enum or struct
    let is_enum = match &input.data {
        Data::Enum(_) => true,
        Data::Struct(_) => false,
        Data::Union(_) => panic!("ModError cannot be derived for unions"),
    };

    // Get the error prefix from attributes
    let error_prefix = get_error_prefix(&input.attrs);

    // Generate implementation based on whether it's an enum or struct
    let implementation = if is_enum {
        implement_for_enum(&input, &error_prefix)
    } else {
        implement_for_struct(&input, &error_prefix)
    };

    // Return the generated implementation
    TokenStream::from(implementation)
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

                display_match_arms.push(quote! {
                    Self::#variant_name { #(#field_names),* } => format!(#display_format, #(#field_names = #field_names),*)
                });

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
                display_match_arms.push(quote! {
                    Self::#variant_name(#(#field_pattern_list)*) => format!(#display_format #(, #field_names)*)
                });

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
