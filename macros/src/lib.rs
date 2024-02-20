

mod as_bind_group_compute;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

// TODO: We dont want to copy the whole thing, just the few changes, but easyer to test this way
// Only modifications so far are is switching ExtractedAssets to ComputeExtractedAssets and the like
// changing paths
#[proc_macro_derive(
    AsBindGroupCompute,
    attributes(uniform, storage_texture, texture, sampler, bind_group_data, storage)
)]
pub fn derive_as_bind_group_compute(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    as_bind_group_compute::derive_as_bind_group_compute(input).unwrap_or_else(|err| err.to_compile_error().into())
}
