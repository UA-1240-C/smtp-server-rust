use proc_macro::TokenStream;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};
use quote::quote;

enum ProcLogLevel {
    Trace,
    Debug,
}

#[proc_macro_attribute]
pub fn log(attr: TokenStream, item: TokenStream) -> TokenStream {
    let log_level = attr.to_string().trim_matches('"').to_lowercase();

    assert!(log_level == "trace" || log_level == "debug", "Invalid log level");
    let log_level = match log_level.as_str() {
        "trace" => ProcLogLevel::Trace,
        "debug" => ProcLogLevel::Debug,
        _ => panic!("Invalid log level"),
    };

    let input_fn: ItemFn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let args = &input_fn.sig.inputs;
    let is_async = input_fn.sig.asyncness.is_some();
    let fn_block = &input_fn.block;

    // Generate logging for arguments, or handle no arguments
    let log_args = {
        let arg_logs = args.iter().filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(ref pat_ident) = *pat_type.pat {
                    let arg_name = &pat_ident.ident;
                    let arg_ty = &pat_type.ty;
                    Some(match log_level {
                        ProcLogLevel::Trace => quote! { format!("{}: {:?}", stringify!(#arg_name), #arg_name) },
                        ProcLogLevel::Debug => quote! { format!("{}: {}", stringify!(#arg_name), stringify!(#arg_ty)) },
                    })
                } else { None }
            } else { None }
        });
        quote! { (vec![#(#arg_logs),*] as Vec<String>).join(", ") } 
    };

    // Handle async and sync functions
    let call_original_fn = if is_async {
        quote! { let result = (async move { #fn_block }).await; }
    } else {
        quote! { let result = (move ||{ #fn_block })(); }
    };

    let module_path = quote! { module_path!() };
    let log_enter = match log_level {
        ProcLogLevel::Trace => quote! { ::logger::trace!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args); },
        ProcLogLevel::Debug => quote! { ::logger::debug!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args); }
    };

    let log_exit = match log_level {
        ProcLogLevel::Trace => quote! { ::logger::trace!("Function {}::{} returned: {:?}", #module_path, stringify!(#fn_name), result); },
        ProcLogLevel::Debug => quote! { ::logger::debug!("Function {}::{} returned.", #module_path, stringify!(#fn_name)); }
    };


    let attributes = &input_fn.attrs;
    let visibility = &input_fn.vis;
    let signature = &input_fn.sig;

    let expanded = quote! {
        #(#attributes)* #visibility #signature {
            #log_enter
            #call_original_fn
            #log_exit
            return result;
        }
    };

    TokenStream::from(expanded)
}