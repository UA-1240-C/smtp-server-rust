use proc_macro::TokenStream;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};
use quote::quote;

#[derive(Eq, PartialEq)]
enum ProcLogLevel {
    Trace,
    Debug,
}

// #[log(trace)] or #[log(debug)]
//
// #[log(trace)] logs the function arguments and return value
// #[log(trace)] is only applicable to function which parameters and return type implement Debug
//
// #[log(debug)] logs the function arguments and their types
//
// Note: If logger level is set to Debug, #[log(trace)] defaults to #[log(debug)]

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


    let log_args_type = args.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ref pat_ident) = *pat_type.pat {
                let pat_ident = &pat_ident.ident;
                let pat_type = &pat_type.ty;
                Some(quote! { format!("{}: {}", stringify!(#pat_ident), stringify!(#pat_type)) })
            } else { None }
        } else { None }
    });


    let log_args_value = args.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ref pat_ident) = *pat_type.pat {
                let pat_ident = &pat_ident.ident;
                Some(quote! { format!("{}: {:?}", stringify!(#pat_ident), #pat_ident) })
            } else { None }
        } else { None }
    });
    
    // string with the function arguments and their types
    let log_args_type = quote! { format!("({})", vec![#(#log_args_type),*].join(", ")) };
    // string with the function arguments and their values
    let log_args_value = quote! { format!("({})", vec![#(#log_args_value),*].join(", ")) };


    let call_original_fn = if is_async {
        quote! { let result = (async move { #fn_block }).await; }
    } else {
        quote! { let result = (move ||{ #fn_block })(); }
    };

    let module_path = quote! { module_path!() };

    let (log_enter, log_exit) = match log_level {
        ProcLogLevel::Trace => (
            quote! { 
                if ::logger::get_logger_level() == ::logger::LogLevel::Trace {
                    ::logger::trace!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_value);
                }
                else {
                    ::logger::debug!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_type);
                }
            },
            quote! { 
                if ::logger::get_logger_level() == ::logger::LogLevel::Trace {
                    ::logger::trace!("Function {}::{} returned: {:?}", #module_path, stringify!(#fn_name), result); 
                }
                else {
                    ::logger::debug!("Function {}::{} returned.", #module_path, stringify!(#fn_name));
                }
            }
        ),
        ProcLogLevel::Debug => (
            quote! { ::logger::debug!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_type); },
            quote! { ::logger::debug!("Function {}::{} returned.", #module_path, stringify!(#fn_name)); }
        ),
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