use proc_macro::TokenStream;
use quote::quote;
use syn::Error;
use syn::spanned::Spanned;

#[proc_macro_derive(UniformIndex)]
pub fn uniform_index(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemEnum);

    if input.variants.len() == 0 {
        return Error::new(
            input.span(),
            "Uniform Shader Indexers must have at least one variant",
        )
        .to_compile_error()
        .into();
    }

    let type_ident = &input.ident;

    let type_ident_str = type_ident.to_string().replace("Uniform", "").replace("Index", "");

    let variants = input.variants.iter().map(|var| &var.ident);
    let index_max = input.variants.len() - 1;
    let amount_addon_impl = match input.variants.len() {
        0 => quote! { impl ::syrillian_utils::ShaderUniformSingleIndex for #type_ident {} },
        _ => quote! { impl ::syrillian_utils::ShaderUniformMultiIndex for #type_ident {} },
    };

    quote! {
        impl ::syrillian_utils::ShaderUniformIndex for #type_ident {
            const MAX: usize = #index_max;

            #[inline]
            fn index(&self) -> u64 {
               *self as u64
            }

            #[inline]
            fn by_index(index: u64) -> Option<Self> {
                index.try_into().ok()
            }

            #[inline]
            fn name() -> &'static str {
                #type_ident_str
            }
        }

        #amount_addon_impl

        impl ::std::convert::TryFrom<u64> for #type_ident {
            type Error = ();
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                match value {
                    #(x if x == Self::#variants as u64 => Ok(Self::#variants),)*
                    _ => Err(()),
                }
            }
        }
    }
    .into()
}

/// This will start a preconfigured runtime for your App. Make sure you have a Default implementation
#[proc_macro_derive(SyrillianApp)]
pub fn syrillian_app(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let logger = cfg!(feature = "derive_env_logger").then(|| {
        quote!(
            ::env_logger::builder()
                .filter_level(::log::LevelFilter::Info)
                .parse_default_env()
                .init();
        )
    });
    
    let app_name = &input.ident;

    quote! {
        #[::syrillian::tokio::main]
        async fn main() {
            let app = ::syrillian::AppRuntime::configure(#app_name::default(), stringify!(#app_name), 800, 600);

            #logger

            if let Err(e) = ::syrillian::AppSettings::run(app).await {
                ::syrillian::log::error!("{e}");
            }
        }
    }.into()
}