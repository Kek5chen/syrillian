use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::Error;

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
    let variants2 = input.variants.iter().map(|var| &var.ident);
    let index_max = input.variants.len() - 1;
    let amount_addon_impl = match input.variants.len() {
        0 => quote! { impl ::syrillian_utils::ShaderUniformSingleIndex for #type_ident {} },
        _ => quote! { impl ::syrillian_utils::ShaderUniformMultiIndex for #type_ident {} },
    };

    quote! {
        impl ::syrillian_utils::ShaderUniformIndex for #type_ident {
            const MAX: usize = #index_max;

            #[inline]
            fn index(&self) -> usize {
               *self as usize
            }

            #[inline]
            fn by_index(index: usize) -> Option<Self> {
                index.try_into().ok()
            }

            #[inline]
            fn name() -> &'static str {
                #type_ident_str
            }
        }

        #amount_addon_impl

        impl ::std::convert::TryFrom<usize> for #type_ident {
            type Error = ();
            fn try_from(value: usize) -> Result<Self, Self::Error> {
                match value {
                    #(x if x == Self::#variants as usize => Ok(Self::#variants),)*
                    _ => Err(()),
                }
            }
        }

        impl ::std::convert::TryFrom<u64> for #type_ident {
            type Error = ();
            fn try_from(value: u64) -> Result<Self, Self::Error> {
                match value {
                    #(x if x == Self::#variants2 as u64 => Ok(Self::#variants2),)*
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
        fn main() {
            let app = ::syrillian::AppRuntime::configure(#app_name::default(), stringify!(#app_name), 800, 600);

            #logger

            if let Err(e) = ::syrillian::AppSettings::run(app) {
                ::syrillian::log::error!("{e}");
            }
        }
    }.into()
}

// TODO: macro-ize some things related to proxy data / scene proxies and in general
// #[proc_macro_attribute]
// fn proxy_data_fn(_: TokenStream, input: TokenStream) -> TokenStream {
//     let func = syn::parse_macro_input!(input as ImplItemFn);
//     match &func.sig.output {
//         ReturnType::Default => wrap_update_render(func),
//         ReturnType::Type(_, _) => wrap_setup_render(func),
//     }.into()
// }
//
// fn wrap_setup_render(mut func: ImplItemFn) -> proc_macro2::TokenStream {
//     let output = &func.sig.output;
//     let ty = match output {
//         ReturnType::Default => unreachable!(),
//         ReturnType::Type(_, ty) => ty,
//     }.clone();
//
//     let new_ident = Ident::new("__inner_setup_render", func.sig.ident.span());
//     func.sig.ident = new_ident.clone();
//
//     quote! {
//         fn setup_render(&mut self, renderer: &Renderer, local_to_world: &Matrix4<f32>) -> Box<dyn Any> {
//             #func
//             let proxy_data = #new_ident(self, renderer, data, window, local_to_world);
//             Box::new(proxy_data)
//         }
//     }
// }
//
// fn wrap_update_render(mut func: ImplItemFn) -> proc_macro2::TokenStream {
//     if func.sig.inputs.len() < 3 {
//         return Error::new_spanned(
//             &func.sig.ident,
//             "expected at least 3 arguments"
//         ).to_compile_error();
//     }
//
//     let third = func.sig.inputs[2].clone();
//
//     let ty = match &third {
//         FnArg::Typed(pat_type) => &*pat_type.ty,
//         FnArg::Receiver(_) => {
//             return Error::new_spanned(
//                 third,
//                 "expected a typed argument, but found `self`"
//             ).to_compile_error().into();
//         }
//     };
//
//     let new_ident = Ident::new("__inner_update_render", func.sig.ident.span());
//     func.sig.ident = new_ident.clone();
//
//     quote! {
//         fn update_render(&mut self, renderer: &Renderer, data: &mut dyn Any, window: &Window, local_to_world: &Matrix4<f32>) {
//             #func
//             let data: #ty = proxy_data_mut!(data);
//
//             #new_ident(self, renderer, data, window, local_to_world)
//         }
//     }
// }