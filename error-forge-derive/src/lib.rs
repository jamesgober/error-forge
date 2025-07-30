extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::{parse_macro_input, DeriveInput, Data, Fields};

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
#[proc_macro_derive(ModError, attributes(error_prefix, error_display, error_kind,
                                         error_caption, error_retryable, error_http_status,
                                         error_exit_code))]
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
            // Try both attribute formats
            // Format: #[error_prefix = "text"]
            if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                if let syn::Lit::Str(lit) = meta.lit {
                    return lit.value();
                }
            }
            // Format: #[error_prefix("text")]
            else if let Ok(syn::Meta::List(meta)) = attr.parse_meta() {
                if let Some(nested) = meta.nested.iter().next() {
                    if let syn::NestedMeta::Lit(syn::Lit::Str(lit)) = nested {
                        return lit.value();
                    }
                }
            }
        }
    }
    String::new()
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
    let mut status_code_match_arms = Vec::new();
    let mut exit_code_match_arms = Vec::new();
    
    // Process each variant
    for variant in &data_enum.variants {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();
        
        // Default values
        let mut display_format = variant_name_str.clone();
        let mut retryable = false;
        let mut status_code: u16 = 500;
        let mut exit_code: i32 = 1;
        
        // Extract attributes
        for attr in &variant.attrs {
            if attr.path.is_ident("error_display") {
                if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                    if let syn::Lit::Str(lit) = meta.lit {
                        display_format = lit.value();
                    }
                }
            } else if attr.path.is_ident("error_retryable") {
                retryable = true;
            } else if attr.path.is_ident("error_http_status") {
                if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                    if let syn::Lit::Int(lit) = meta.lit {
                        status_code = lit.base10_parse().unwrap_or(500);
                    }
                }
            } else if attr.path.is_ident("error_exit_code") {
                if let Ok(syn::Meta::NameValue(meta)) = attr.parse_meta() {
                    if let syn::Lit::Int(lit) = meta.lit {
                        exit_code = lit.base10_parse().unwrap_or(1);
                    }
                }
            }
        }
        
        // Generate pattern matching based on the variant's fields
        match &variant.fields {
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                
                // Format string handled directly in match arm
                
                kind_match_arms.push(quote! {
                    Self::#variant_name { .. } => #variant_name_str
                });
                
                caption_match_arms.push(quote! {
                    Self::#variant_name { .. } => concat!(#error_prefix, ": Error")
                });
                
                let _field_patterns = field_names.iter().map(|name| {
                    let _name_str = name.to_string();
                    quote! { #name, }
                });
                
                // For struct variants, create a properly formatted string without using fields
                display_match_arms.push(quote! {
                    Self::#variant_name { .. } => format!("{}: {}", #error_prefix, #display_format)
                });
                
                retryable_match_arms.push(quote! {
                    Self::#variant_name { .. } => #retryable
                });
                
                status_code_match_arms.push(quote! {
                    Self::#variant_name { .. } => #status_code
                });
                
                exit_code_match_arms.push(quote! {
                    Self::#variant_name { .. } => #exit_code
                });
            },
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_names: Vec<_> = (0..field_count)
                    .map(|i| format_ident!("_{}", i))
                    .collect();
                
                // Generate display format with tuple fields
                // field_names is already Vec<Ident> so we can pass it directly
                // Format string handled directly in match arm
                
                kind_match_arms.push(quote! {
                    Self::#variant_name(..) => #variant_name_str
                });
                
                caption_match_arms.push(quote! {
                    Self::#variant_name(..) => concat!(#error_prefix, ": Error")
                });
                
                let _field_patterns = field_names.iter().map(|name| {
                    quote! { #name, }
                });
                
                // For tuple variants, handle simple positional formatting for {0}, {1}, etc.
                if display_format.contains("{0}") || display_format.contains("{}") {
                    // Recreate the field pattern list here to avoid conflicts with renamed variables
                    let field_pattern_list = field_names.iter().map(|name| quote! { #name, });
                    display_match_arms.push(quote! {
                        Self::#variant_name(#(#field_pattern_list)*) => format!("{}: {}", #error_prefix, format!(#display_format #(, #field_names)*))
                    });
                } else {
                    // Fall back to simple display if no formatting placeholders
                    display_match_arms.push(quote! {
                        Self::#variant_name(..) => format!("{}: {}", #error_prefix, #display_format)
                    });
                }
                
                retryable_match_arms.push(quote! {
                    Self::#variant_name(..) => #retryable
                });
                
                status_code_match_arms.push(quote! {
                    Self::#variant_name(..) => #status_code
                });
                
                exit_code_match_arms.push(quote! {
                    Self::#variant_name(..) => #exit_code
                });
            },
            Fields::Unit => {
                // Unit variant (no fields)
                kind_match_arms.push(quote! {
                    Self::#variant_name => #variant_name_str
                });
                
                caption_match_arms.push(quote! {
                    Self::#variant_name => concat!(#error_prefix, ": Error")
                });
                
                display_match_arms.push(quote! {
                    Self::#variant_name => format!("{}: {}", #error_prefix, #display_format)
                });
                
                retryable_match_arms.push(quote! {
                    Self::#variant_name => #retryable
                });
                
                status_code_match_arms.push(quote! {
                    Self::#variant_name => #status_code
                });
                
                exit_code_match_arms.push(quote! {
                    Self::#variant_name => #exit_code
                });
            },
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
