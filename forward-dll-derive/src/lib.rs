use object::Object;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, LitStr};

#[proc_macro_derive(ForwardModule, attributes(forward))]
pub fn derive_forward_module(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::DeriveInput);
    let forward_attr = input
        .attrs
        .iter()
        .find(|i| i.path().is_ident("forward"))
        .unwrap();
    let dll_path: LitStr = forward_attr.parse_args().unwrap();
    let struct_name = input.ident;

    let export_names = get_dll_export_names(dll_path.value().as_str());
    let export_idents: Vec<_> = export_names
        .iter()
        .map(|i| format_ident!("{}", i))
        .collect();
    let export_count = export_names.len();

    let impl_code = quote! {
        const _ : () = {
            extern crate forward_dll as _forward_dll;

            static mut _FORWARDER: _forward_dll::DllForwarder<#export_count> = _forward_dll::DllForwarder {
                lib_name: #dll_path,
                target_functions_address: [0; #export_count],
                target_function_names: [#(#export_names),*],
            };

            _forward_dll::define_function!(#dll_path, _FORWARDER, 0, #(#export_idents)*);

            impl _forward_dll::ForwardModule for #struct_name {
                fn init(&self) -> _forward_dll::ForwardResult<()> {
                    let _ = _forward_dll::load_library(#dll_path);
                    unsafe { _FORWARDER.forward_all() }
                }
            }
        };
    };
    impl_code.into()
}

fn get_dll_export_names(dll_path: &str) -> Vec<String> {
    let dll_file = std::fs::read(dll_path).unwrap();
    let pe = object::File::parse(&*dll_file).unwrap();
    let exports = pe.exports().unwrap();
    let mut names = Vec::new();
    for export_item in exports {
        names.push(String::from_utf8_lossy(export_item.name()).into());
    }
    names
}
