extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(Bundle, attributes(folder))]
pub fn bundle(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let name = &input.ident;

    let mut folder = None;
    for attr in input.attrs {
        if let Ok(syn::Meta::NameValue(meta_name_value)) = attr.parse_meta() {
            let syn::MetaNameValue { path, lit, .. } = meta_name_value;
            let is_folder_attr = path.get_ident().unwrap().to_string() == "folder";
            if let syn::Lit::Str(lit) = lit {
                if is_folder_attr {
                    folder = Some(lit.value());
                } else {
                    let msg = r#"Bad path! should be similar to #[folder = "dist/"]"#;
                    return syn::Error::new_spanned(&attr, msg)
                        .to_compile_error()
                        .into();
                }
            } else {
                let msg = "folder path provided was not a string literal!";
                return syn::Error::new_spanned(&attr, msg)
                    .to_compile_error()
                    .into();
            }
        }
    }

    let folder = folder.expect(r#"No path provided, should be similar to #[folder = "dist/"]"#);

    let window_mode = if cfg!(feature = "nowindow") {
        quote! {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
    } else {
        quote! {}
    };

    let expanded = quote! {

        #[derive(rust_embed::RustEmbed)]
        #[folder = #folder]
        struct __Asset;


        struct __Packed {
            temp_dir: tempfile::TempDir,
            exe_path: ::std::option::Option<::std::path::PathBuf>,
        }

        impl __Packed {
            fn new() -> Self {
                Self {
                    temp_dir: tempfile::TempDir::new().expect("Could not create temp dir"),
                    exe_path: None,
                }
            }

            fn dump(&mut self) {

                for file in __Asset::iter() {
                    let file_path = ::std::path::Path::new(file.as_ref());

                    if file_path.extension().expect("Could not get filename") == "exe" {
                        self.exe_path = Some(file_path.to_path_buf())
                    };

                    let path = self.temp_dir.path().join(file_path);

                    let folders = path.parent().expect("Could not get parent");

                    if !folders.to_str().expect("Not unicode!").is_empty() {
                        ::std::fs::create_dir_all(folders)
                            .expect("Could not create dirs recursively for embedded files");
                    }

                    let data = __Asset::get(file_path.to_str().expect("File path is not unicode"))
                        .expect("Could not get the asset");

                    ::std::fs::write(path, data).expect("Writing in temp directory failed");
                }
            }

            fn launch(&self) {
                if let Some(exe_path) = &self.exe_path {
                    __execute(self.temp_dir.path(), exe_path);
                } else {
                    eprintln!("No executable found!");
                }
            }
        }

        impl #name {
            fn run() {
                let mut packed = __Packed::new();
                packed.dump();
                packed.launch();
            }
        }

        fn __execute(temp_dir: &::std::path::Path, exe_path: &::std::path::Path) {
            use ::std::os::windows::process::CommandExt;

            ::std::env::set_current_dir(temp_dir).expect("Could not change directory");
            let mut cmd = ::std::process::Command::new(exe_path);

            #window_mode

            let mut child = cmd.spawn().expect("Could not spawn command");
            child.wait().expect("command wasn't running");
        }

    };

    expanded.into()
}
