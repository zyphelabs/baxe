use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Parser, spanned::Spanned, Data, DeriveInput, Expr, Fields};

#[proc_macro_attribute]
pub fn error(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

fn expand(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut log_message_with = None;
    let mut hide_message = false;

    syn::meta::parser(|meta| {
        if meta.path.is_ident("logMessageWith") {
            log_message_with = Some(meta.value()?.parse::<Expr>()?);
        } else if meta.path.is_ident("hideMessage") {
            hide_message = true;
        } else if meta.input.peek(syn::Token![=]) {
            // Preserve support for ignored, unknown options.
            let _ = meta.value()?.parse::<Expr>()?;
        }
        Ok(())
    })
    .parse2(attr)?;

    let input: DeriveInput = syn::parse2(item)?;

    for attr in &input.attrs {
        if attr.path().is_ident("baxe") {
            let err = syn::Error::new(
                attr.span(),
                "The #[baxe(...)] attribute is only allowed on enum variants, not on the enum itself.",
            );
            return Err(err);
        }
    }

    let enum_name = input.ident;
    let data = match input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new(
                enum_name.span(),
                "baxe::error can only be applied to enums",
            ))
        }
    };

    let variants_def = data
        .variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            match &v.fields {
                Fields::Unit => quote! { #variant_ident },
                Fields::Unnamed(fields) => {
                    let types = fields.unnamed.iter().map(|f| &f.ty);
                    quote! { #variant_ident(#(#types),*) }
                }
                Fields::Named(fields) => {
                    let field_defs = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let ty = &f.ty;
                        quote! { #name: #ty }
                    });
                    quote! { #variant_ident { #(#field_defs),* } }
                }
            }
        })
        .collect::<Vec<_>>();

    let count = data.variants.len();
    let mut patterns = Vec::with_capacity(count);
    let mut statuses = Vec::with_capacity(count);
    let mut tags = Vec::with_capacity(count);
    let mut codes = Vec::with_capacity(count);
    let mut messages = Vec::with_capacity(count);

    for variant in &data.variants {
        let variant_ident = &variant.ident;
        let attrs = parse_baxe_attributes(variant)?;
        let message = attrs.message;
        let (pattern, bindings) = match &variant.fields {
            Fields::Unit => (quote! { #enum_name::#variant_ident }, Vec::new()),
            Fields::Unnamed(fields) => {
                let bindings: Vec<_> = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        syn::Ident::new(&format!("arg{i}"), proc_macro2::Span::call_site())
                    })
                    .collect();
                (
                    quote! { #enum_name::#variant_ident(#(#bindings),*) },
                    bindings,
                )
            }
            Fields::Named(fields) => {
                let bindings: Vec<_> = fields
                    .named
                    .iter()
                    .map(|field| field.ident.clone().unwrap())
                    .collect();
                (
                    quote! { #enum_name::#variant_ident { #(#bindings),* } },
                    bindings,
                )
            }
        };

        messages.push(quote! {
            #pattern => write!(f, #message #(, #bindings)*)
        });
        patterns.push(pattern);
        statuses.push(attrs.status);
        tags.push(attrs.tag);
        codes.push(attrs.code);
    }

    let log_statement = if let Some(log_fn) = log_message_with {
        quote! {
            #log_fn!("{}", error.to_string());
        }
    } else {
        quote! {}
    };

    let to_message = if hide_message {
        quote! {
            None
        }
    } else {
        quote! {
            error.to_string().into()
        }
    };

    let expanded = quote! {
        #[derive(Debug)]
        pub enum #enum_name {
            #(#variants_def,)*
        }

        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#messages,)*
                }
            }
        }

        impl std::error::Error for #enum_name {}

        impl BackendError for #enum_name {
            fn to_status_code(&self) -> axum::http::StatusCode {
                match self {
                    #(#patterns => #statuses,)*
                }
            }

            fn to_error_tag(&self) -> impl std::fmt::Display {
                match self {
                    #(#patterns => #tags,)*
                }
            }

            fn to_error_code(&self) -> u16 {
                match self {
                    #(#patterns => #codes,)*
                }
            }
        }

        impl From<#enum_name> for BaxeError {
            fn from(error: #enum_name) -> Self {
                #log_statement
                let status = error.to_status_code();
                let tag: String = error.to_error_tag().to_string();
                BaxeError::new(status, #to_message, error.to_error_code(), tag)
            }
        }

        impl IntoResponse for #enum_name {
            fn into_response(self) -> axum::response::Response {
                (self.to_status_code(), Json(BaxeError::from(self))).into_response()
            }
        }
    };

    Ok(expanded)
}

struct BaxeAttributes {
    status: proc_macro2::TokenStream,
    tag: proc_macro2::TokenStream,
    code: proc_macro2::TokenStream,
    message: proc_macro2::TokenStream,
}

fn parse_baxe_attributes(variant: &syn::Variant) -> syn::Result<BaxeAttributes> {
    let mut attrs = BaxeAttributes {
        status: quote!(None),
        tag: quote!(None),
        code: quote!(None),
        message: quote!(None),
    };

    for attr in &variant.attrs {
        if attr.path().is_ident("baxe") {
            attr.parse_nested_meta(|meta| {
                let value = meta.value()?.parse::<Expr>()?;
                if meta.path.is_ident("status") {
                    attrs.status = quote!(#value);
                } else if meta.path.is_ident("tag") {
                    attrs.tag = quote!(#value);
                } else if meta.path.is_ident("code") {
                    attrs.code = quote!(#value);
                } else if meta.path.is_ident("message") {
                    attrs.message = quote!(#value);
                }
                Ok(())
            })?;
        }
    }

    Ok(attrs)
}

#[cfg(test)]
mod tests;
