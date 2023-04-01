use object::read::pe::{PeFile32, PeFile64};
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
        .expect(r#"你需要添加 #[forward(target = "path/of/target_dll.dll")]"#);

    // 解析 #[forward(target = "", ordinal)] 的参数。
    let mut dll_path: Option<LitStr> = None;
    let mut has_ordinal = false;
    forward_attr
        .parse_nested_meta(|meta| {
            let path = &meta.path;
            if path.is_ident("target") {
                let value = meta.value().unwrap();
                dll_path = Some(value.parse().unwrap());
            } else if path.is_ident("ordinal") {
                has_ordinal = true;
            }
            Ok(())
        })
        .expect("测试两下");

    let dll_path = dll_path.expect(
        r#"#[forward()] 的 target 参数为必填项，如 #[forward(target = "C:\Windows\System32\version.dll")]"#,
    );
    let exports = get_dll_export_names(dll_path.value().as_str())
        .expect("指定的 DLL 可能是一个无效的 PE 文件");

    // 生成 /EXPORT:EntryName 的编译器参数。
    if has_ordinal {
        generate_linker_args(&exports);
    }

    let export_names: Vec<_> = exports.iter().map(|(_, fn_name)| fn_name).collect();
    let export_definitions: Vec<_> = exports
        .iter()
        .map(|(_, fn_name)| {
            let export_name = match has_ordinal {
                true => format_ident!("_{}", fn_name),
                false => format_ident!("{}", fn_name),
            };
            let fn_name = format_ident!("{}", fn_name);
            quote! {
                #export_name = #fn_name
            }
        })
        .collect();
    let export_count = exports.len();
    let struct_name = input.ident;
    let impl_code = quote! {
        const _ : () = {
            extern crate forward_dll as _forward_dll;

            static mut _FORWARDER: _forward_dll::DllForwarder<#export_count> = _forward_dll::DllForwarder {
                initialized: false,
                module_handle: 0,
                lib_name: #dll_path,
                target_functions_address: [0; #export_count],
                target_function_names: [#(#export_names),*],
            };

            _forward_dll::define_function!(#dll_path, _FORWARDER, 0, #(#export_definitions)*);

            impl _forward_dll::ForwardModule for #struct_name {
                fn init(&self) -> _forward_dll::ForwardResult<()> {
                    unsafe { _FORWARDER.forward_all() }
                }
            }
        };
    };
    impl_code.into()
}

fn get_dll_export_names(dll_path: &str) -> Result<Vec<(u32, String)>, String> {
    let dll_file = std::fs::read(dll_path).unwrap();
    let in_data = dll_file.as_slice();

    let kind = object::FileKind::parse(in_data).map_err(|err| format!("Invalid file: {err}"))?;
    let exports = match kind {
        object::FileKind::Pe32 => PeFile32::parse(in_data)
            .unwrap()
            .export_table()
            .unwrap()
            .unwrap()
            .exports(),
        object::FileKind::Pe64 => PeFile64::parse(in_data)
            .unwrap()
            .export_table()
            .unwrap()
            .unwrap()
            .exports(),
        _ => return Err("Invalid file".to_string()),
    }
    .map_err(|err| format!("Invalid file: {err}"))?;

    let mut names = Vec::new();
    for export_item in exports {
        names.push((
            export_item.ordinal,
            export_item
                .name
                .map(String::from_utf8_lossy)
                .map(String::from)
                .unwrap_or_default(),
        ));
    }
    Ok(names)
}

fn generate_linker_args(exports: &Vec<(u32, String)>) {
    let out_dir: std::path::PathBuf = std::path::PathBuf::from(env!("OUT_DIR"))
        .components()
        .rev()
        .skip_while(|path| {
            let path = path.as_os_str().to_str().unwrap_or_default();
            path == "out" || path.contains("forward-dll-derive") || path == "build"
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if out_dir.is_dir() {
        let ordinal_content = exports
            .iter()
            .map(|(ordinal, fn_name)| format!("/EXPORT:{}=_{},@{}", fn_name, fn_name, ordinal))
            .collect::<Vec<_>>()
            .join("\n");
        let ordinal_file = out_dir.join("ordinal_link_args.txt");
        let _ = std::fs::write(ordinal_file, ordinal_content);
    }
}
