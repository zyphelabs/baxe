use super::expand;
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::{hint::black_box, time::Instant};

fn fixture(count: usize) -> TokenStream {
    let variants = (0..count).map(|i| {
        let name = format_ident!("Error{i}");
        quote! {
            #[baxe(status = StatusCode::BAD_REQUEST, tag = "bad_request", code = 400,
                   message = "Invalid value {0}: {1:?}")]
            #name(String, Vec<usize>)
        }
    });
    quote! { enum Errors { #(#variants,)* } }
}

fn token_count(tokens: TokenStream) -> usize {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Group(group) => 1 + token_count(group.stream()),
            _ => 1,
        })
        .sum()
}

// Run with: cargo test -p baxe-derive --release benchmark_expansion -- --ignored --nocapture
// This measures parsing + code generation using proc_macro2's fallback backend;
// it excludes the compiler's proc-macro bridge and expansion of generated macros.
#[test]
#[ignore = "manual performance measurement"]
fn benchmark_expansion() {
    for count in [10, 100, 500] {
        let input = fixture(count);
        let output_tokens = token_count(expand(TokenStream::new(), input.clone()).unwrap());
        let iterations = 10_000 / count;
        for _ in 0..20 {
            black_box(expand(TokenStream::new(), input.clone()).unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            for _ in 0..iterations {
                black_box(expand(TokenStream::new(), black_box(input.clone())).unwrap());
            }
            samples.push(start.elapsed().as_secs_f64() * 1_000_000.0 / iterations as f64);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "variants={count} median_us={:.1} min_us={:.1} max_us={:.1} tokens={output_tokens}",
            samples[4], samples[0], samples[8]
        );
    }
}

#[test]
fn malformed_options_return_diagnostics() {
    assert!(expand(quote!(logMessageWith =), fixture(1)).is_err());
    assert!(expand(
        quote!(logMessageWith = logger::error hideMessage),
        fixture(1)
    )
    .is_err());
}

#[test]
fn invalid_input_returns_diagnostics() {
    assert!(expand(
        TokenStream::new(),
        quote!(
            struct NotAnEnum;
        )
    )
    .is_err());
    assert!(expand(
        TokenStream::new(),
        quote! {
            #[baxe(code = 400)] enum Errors { Bad }
        }
    )
    .is_err());
    assert!(expand(
        TokenStream::new(),
        quote! {
            enum Errors { #[baxe(code =)] Bad }
        }
    )
    .is_err());
}
