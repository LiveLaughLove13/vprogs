mod smart_pointer;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

use crate::smart_pointer::{expand_smart_pointer, Args};

#[proc_macro_attribute]
pub fn smart_pointer(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Args);
    let item = parse_macro_input!(input as ItemStruct);

    match expand_smart_pointer(item, args) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
