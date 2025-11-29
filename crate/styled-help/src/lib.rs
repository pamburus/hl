use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, DeriveInput, Field, Lit, Meta, parse_macro_input};

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
    // Extract and combine all doc comments
    let mut doc_lines = Vec::new();
    let mut has_existing_help = false;

    // Check if field already has a help or long_help attribute
    for attr in &field.attrs {
        if attr.path().is_ident("arg") {
            if let Meta::List(ref meta_list) = attr.meta {
                // Parse the tokens inside the arg attribute
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("help") || tokens_str.contains("long_help") {
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
    field.attrs.retain(|attr| {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(ref meta) = attr.meta {
                if let syn::Expr::Lit(ref expr_lit) = meta.value {
                    if let Lit::Str(ref lit_str) = expr_lit.lit {
                        let doc_content = lit_str.value();
                        doc_lines.push(doc_content.trim().to_string());
                        return false; // Remove the doc attribute
                    }
                }
            }
        }
        true // Keep non-doc attributes
    });

    // If no doc comments found, nothing to do
    if doc_lines.is_empty() {
        return;
    }

    // Combine doc lines into a single string
    let combined_doc = doc_lines.join("\n");

    // Check if the doc comment contains style markers
    let has_style_markers = combined_doc.contains("<c>")
        || combined_doc.contains("</>")
        || combined_doc.contains("<s>")
        || combined_doc.contains("<u>")
        || combined_doc.contains("<k>")
        || combined_doc.contains("<r>")
        || combined_doc.contains("<g>")
        || combined_doc.contains("<b>")
        || combined_doc.contains("<y>")
        || combined_doc.contains("<m>")
        || combined_doc.contains("<cyan>")
        || combined_doc.contains("<white>");

    // Create the help attribute
    if has_style_markers {
        // Use cstr! for styled help
        let help_attr: Attribute = syn::parse_quote! {
            #[arg(help = ::color_print::cstr!(#combined_doc))]
        };
        field.attrs.push(help_attr);
    } else {
        // Use plain string for non-styled help
        let help_attr: Attribute = syn::parse_quote! {
            #[arg(help = #combined_doc)]
        };
        field.attrs.push(help_attr);
    }
}
