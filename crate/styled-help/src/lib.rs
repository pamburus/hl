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

    // Collect doc comments and check for style markers
    let mut doc_lines = Vec::new();
    let mut has_style_markers = false;

    for attr in &field.attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(ref meta) = attr.meta {
                if let syn::Expr::Lit(ref expr_lit) = meta.value {
                    if let Lit::Str(ref lit_str) = expr_lit.lit {
                        let doc_content = lit_str.value();
                        doc_lines.push(doc_content.trim().to_string());

                        // Check for style markers
                        if !has_style_markers {
                            has_style_markers = doc_content.contains("<c>")
                                || doc_content.contains("</>")
                                || doc_content.contains("<s>")
                                || doc_content.contains("<u>")
                                || doc_content.contains("<k>")
                                || doc_content.contains("<r>")
                                || doc_content.contains("<g>")
                                || doc_content.contains("<b>")
                                || doc_content.contains("<y>")
                                || doc_content.contains("<m>")
                                || doc_content.contains("<cyan>")
                                || doc_content.contains("<white>");
                        }
                    }
                }
            }
        }
    }

    // If no doc comments found or no style markers, let clap handle it normally
    if doc_lines.is_empty() || !has_style_markers {
        return;
    }

    // Only process and remove doc comments if they have style markers
    field.attrs.retain(|attr| !attr.path().is_ident("doc"));

    // Combine doc lines into a single string
    let combined_doc = doc_lines.join("\n");

    // Split into paragraphs (separated by empty lines)
    // Clap's behavior:
    // - Short help (-h): First paragraph only, strip trailing period
    // - Long help (--help): Full text, keep periods as-is
    let paragraphs: Vec<&str> = combined_doc.split("\n\n").collect();

    let short_help = if let Some(first_para) = paragraphs.first() {
        // Strip trailing period from first paragraph for short help
        let trimmed = first_para.trim_end();
        if trimmed.ends_with('.') {
            trimmed[..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        String::new()
    };

    let long_help = combined_doc;

    // Use cstr! for styled help
    let help_attr: Attribute = syn::parse_quote! {
        #[arg(help = ::color_print::cstr!(#short_help), long_help = ::color_print::cstr!(#long_help))]
    };
    field.attrs.push(help_attr);
}
