#![feature(proc_macro_quote)]

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, FieldsNamed};


#[proc_macro_derive(Comprezable, attributes(maxNum))]
pub fn derive_describe_fn(_item: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(_item);
    let mut compress_tokens = vec![];
    let mut max_binaries_tokens = vec![];
    let mut decompress_tokens = vec![];
    match data {
        syn::Data::Struct(s) => match s.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => {
                for field in named {
                    let data_type = field.ty.clone().into_token_stream();
                    let data_name = field.ident.to_token_stream();
                    let data_attr = field.attrs;
                    let data_type_str = field.ty.clone().into_token_stream().to_string();

                    let mut max_num = 0;
                    let attrs_str = data_attr.iter().map(|attr| attr.to_token_stream().to_string()).filter(|attr| attr.contains("maxNum")).collect::<Vec<String>>();
                    if attrs_str.len() != 1 {
                        match data_type_str.as_str() {
                            "u8" | "u16" | "u32" | "u64" | "u128" => panic!("Each field in the struct must have only ONE 'maxNum' attribute"),
                            _ => (),
                        }
                        
                    }

                    for attr in attrs_str {
                        let equal_index = attr.find('=').ok_or_else(|| panic!("Error finding '=' in the attribute: {}", attr)).unwrap();
                        let (_, num) = attr.split_at(equal_index + 1);
                        max_num = num.trim_end_matches("]").trim().parse::<u128>().map_err(|_| {
                            panic!("Error parsing maxNum: {}", num.trim())
                        }).unwrap();
                    }

                    let compress_token = quote! {
                        let #data_name = self.#data_name.compress(Some(#max_num))?;
                        all_compressed.push(#data_name);
                    };

                    let max_binaries_token = quote! {
                        let #data_name = #data_type::max_binaries(Some(#max_num));
                        size.push(#data_name);
                    };

                    let decompress_token = quote! {
                        #data_name: #data_type::decompress(Compressed::from_binaries(chunked_binaries.next().unwrap()))?
                    };


                    compress_tokens.push(compress_token);
                    max_binaries_tokens.push(max_binaries_token);
                    decompress_tokens.push(decompress_token);
                }
            },
            _ => panic!(),
        },
        _ => panic!(),
    };
    //let tokens = tokens.iter();
    let compress_tokens = compress_tokens.iter();
    let max_binaries_tokens = max_binaries_tokens.iter();
    let decompress_tokens = decompress_tokens.iter();
    let output = quote! {
        
        impl Comprezable for #ident {

            fn compress(self, _max_num: Option<u128>) -> Result<Compressed, CompressError> {
                let mut all_compressed: Vec<Compressed> = vec![];
                #(#compress_tokens)*
        
                let mut binaries = Compressed::Binaries(vec![]);
                for compressed in all_compressed {
                    binaries = binaries.combine(compressed);
                }
                
                Ok(Compressed::Bytes(binaries.to_bytes()))
            }
        
            fn max_binaries(_max_num: Option<u128>) -> BinaryChunk {
                let mut size = vec![];
                #(#max_binaries_tokens)*
                
                BinaryChunk::Nested(size)
            }
        
            fn decompress(compressed: Compressed) -> Result<#ident, DecompressError> {
                let binary_chunks = #ident::max_binaries(None);
                let binaries = compressed.to_binaries();
                let mut chunk_sizes = Vec::new();
                match binary_chunks {
                    BinaryChunk::Nested(chunks) => {
                        for chunk in chunks {
                            let chunk_size = chunk.flatten().iter().sum::<usize>();
                            chunk_sizes.push(chunk_size);
                        }
                    },
                    BinaryChunk::Single(_) => {
                        panic!()
                    }
                }
                
                let sum_of_chunks = chunk_sizes.iter().sum::<usize>();
                let mut chunked_binaries = chunk_up(binaries.as_slice(), chunk_sizes.as_slice())
                .map_err(|_| {
                    DecompressError::create(DecompressError::WrongBytesLength(format!("given: {}-length binaries, should be: {}-length binaries; ", sum_of_chunks, binaries.len())))
                })?.into_iter();
                
                Ok(
                    #ident {
                        #(#decompress_tokens), *
                    }
                )
            }

            
        }
    };

    output.into()
}


