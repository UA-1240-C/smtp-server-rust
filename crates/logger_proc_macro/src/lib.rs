//! # Log Procedural Macro
//! This module defines a procedural macro, `#[log]`, which allows logging function 
//! calls and their parameters or return values based on the specified log level. 
//! It supports two log levels: `trace` and `debug`.
//!
//! ## Usage
//! - `#[log(trace)]`: Logs function arguments and return values. This requires all
//!   function parameters and return types to implement the `Debug` trait.
//! - `#[log(debug)]`: Logs function arguments and their types, without requiring 
//!   `Debug` on the values.
//!
//! If the logger's severity level is set to `Debug`, then `#[log(trace)]` will behave 
//! like `#[log(debug)]`, logging argument types instead of values. 

use proc_macro::TokenStream;
use syn::{parse_macro_input, FnArg, ItemFn, Pat};
use quote::quote;

#[derive(Eq, PartialEq)]
enum ProcLogLevel {
    Trace,
    Debug,
}

/// Procedural macro for logging function calls.
///
/// This macro inspects function parameters and logs either their values or types, 
/// depending on the log level specified in the attribute. 
/// It works for both synchronous and asynchronous functions.
///
/// ## Parameters
/// - `trace`: Logs the function arguments and return value, provided all arguments
///   and return types implement `Debug`.
/// - `debug`: Logs the function arguments and their types.
///
/// ## Behavior
/// - If the logger's level is `Trace`, the macro logs the argument values and the return value.
/// - If the logger's level is `Debug`, it defaults to logging argument types only.


#[proc_macro_attribute]
pub fn log(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the log level from the attribute
    let log_level = attr.to_string().trim_matches('"').to_lowercase();
    assert!(log_level == "trace" || log_level == "debug", "Invalid log level");

    // Determine the log level
    let log_level = match log_level.as_str() {
        "trace" => ProcLogLevel::Trace,
        "debug" => ProcLogLevel::Debug,
        _ => panic!("Invalid log level"),
    };

    // Parse the input function
    let input_fn: ItemFn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;  // Function name
    let args = &input_fn.sig.inputs;    // Function arguments
    let is_async = input_fn.sig.asyncness.is_some();  // Check if the function is asynchronous
    let fn_block = &input_fn.block;     // Function body

    // Prepare the logging for argument types
    let log_args_type = args.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            // Extract the argument name and type
            if let Pat::Ident(ref pat_ident) = *pat_type.pat {
                let pat_ident = &pat_ident.ident;
                let pat_type = &pat_type.ty;
                // Format the argument name and its type
                Some(quote! { format!("{}: {}", stringify!(#pat_ident), stringify!(#pat_type)) })
            } else { None }
        } else { None }
    });

    // Prepare the logging for argument values
    let log_args_value = args.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            // Extract the argument name
            if let Pat::Ident(ref pat_ident) = *pat_type.pat {
                let pat_ident = &pat_ident.ident;
                // Format the argument name and its value using Debug trait
                Some(quote! { format!("{}: {:?}", stringify!(#pat_ident), #pat_ident) })
            } else { None }
        } else { None }
    });

    // Combine the argument types into a single string
    let log_args_type = quote! { format!("({})", (vec![#(#log_args_type),*] as Vec<String>).join(", ")) };
    // Combine the argument values into a single string
    let log_args_value = quote! { format!("({})", (vec![#(#log_args_value),*] as Vec<String>).join(", ")) };

    // Generate the call to the original function, handling async/sync functions
    let call_original_fn = if is_async {
        quote! { let result = (async move { #fn_block }).await; }
    } else {
        quote! { let result = (move ||{ #fn_block })(); }
    };

    // Capture the current module path for logging
    let module_path = quote! { module_path!() };

    // Create log messages for function entry and exit, depending on the log level
    let (log_enter, log_exit) = match log_level {
        ProcLogLevel::Trace => (
            // Trace level: Log argument values and return value
            quote! { 
                if ::logger::get_logger_level() == ::logger::LogLevel::Trace {
                    ::logger::trace!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_value);
                } else {
                    ::logger::debug!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_type);
                }
            },
            quote! { 
                if ::logger::get_logger_level() == ::logger::LogLevel::Trace {
                    ::logger::trace!("Function {}::{} returned: {:?}", #module_path, stringify!(#fn_name), result); 
                } else {
                    ::logger::debug!("Function {}::{} returned.", #module_path, stringify!(#fn_name));
                }
            }
        ),
        ProcLogLevel::Debug => (
            quote! { ::logger::debug!("Function call {}::{}({})", #module_path, stringify!(#fn_name), #log_args_type); },
            quote! { ::logger::debug!("Function {}::{} returned.", #module_path, stringify!(#fn_name)); }
        ),
    };

    // Extract function attributes, visibility, and signature
    let attributes = &input_fn.attrs;
    let visibility = &input_fn.vis;
    let signature = &input_fn.sig;

    // Expand the original function with the logging behavior
    let expanded = quote! {
        #(#attributes)* #visibility #signature {
            #log_enter          // Log function entry
            #call_original_fn    // Call the original function
            #log_exit            // Log function exit
            return result;       // Return the function result
        }
    };

    // Convert the expanded code back to a TokenStream
    TokenStream::from(expanded)
}
