use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, DeriveInput, Field, Lit, Meta};

/// Transforms doc comments with color_print style markers into `help` attributes.
///
/// This macro processes doc comments on struct fields and converts them to `help` attributes
/// that use `color_print::cstr!` for styling. This allows you to use markers like `<c>text</>`
/// directly in doc comments.
///
/// # Example
///
/// ```ignore
/// use clap::Parser;
/// use styled_help::styled_help;
///
/// #[styled_help]
/// #[derive(Parser)]
/// struct Opt {
///     /// Sort messages using <c>--sync-interval-ms</> option
///     #[arg(long)]
///     sort: bool,
/// }
/// ```
///
/// The doc comment will be transformed into:
/// `#[arg(long, help = color_print::cstr!("Sort messages using <c>--sync-interval-ms</> option"))]`
#[proc_macro_attribute]
pub fn styled_help(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as DeriveInput);

    if let syn::Data::Struct(ref mut data_struct) = input.data {
        if let syn::Fields::Named(ref mut fields) = data_struct.fields {
            for field in fields.named.iter_mut() {
                process_field(field);
            }
        }
    }

    TokenStream::from(quote! { #input })
}

fn process_field(field: &mut Field) {
    // Check if field already has a help or long_help attribute
    let mut has_existing_help = false;
    for attr in &field.attrs {
        if attr.path().is_ident("arg") {
            if let Meta::List(ref meta_list) = attr.meta {
                // Parse the tokens inside the arg attribute
                let tokens_str = meta_list.tokens.to_string();
                // Check for "help =" or "long_help =" to avoid matching help_heading
                if tokens_str.contains("help =") || tokens_str.contains("long_help =") {
                    has_existing_help = true;
                    break;
                }
            }
        }
    }

    // If there's already a help attribute, don't process doc comments
    if has_existing_help {
        return;
    }

    // Collect doc comments
    let mut doc_lines = Vec::new();

    for attr in &field.attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(ref meta) = attr.meta {
                if let syn::Expr::Lit(ref expr_lit) = meta.value {
                    if let Lit::Str(ref lit_str) = expr_lit.lit {
                        let doc_content = lit_str.value();
                        doc_lines.push(doc_content.trim().to_string());
                    }
                }
            }
        }
    }

    // If no doc comments found, nothing to do
    if doc_lines.is_empty() {
        return;
    }

    // Process all doc comments (remove them to avoid duplication with help attributes)
    field.attrs.retain(|attr| !attr.path().is_ident("doc"));

    // Combine doc lines into a single string
    let long_help = doc_lines.join("\n");

    // Strip trailing period only from help (short help), keep it in long_help
    let help = if long_help.ends_with('.') {
        long_help[..long_help.len() - 1].to_string()
    } else {
        long_help.clone()
    };

    // Generate help attributes - always use cstr! (it handles plain text just fine)
    let help_attr: Attribute = syn::parse_quote! {
        #[arg(help = ::color_print::cstr!(#help), long_help = ::color_print::cstr!(#long_help))]
    };
    field.attrs.push(help_attr);
}
