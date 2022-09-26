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
                    let data_name = field.ident.to_token_stream();
                    let mut data_type = field.ty.clone().into_token_stream();
                    let data_attr = field.attrs;
                    let data_type_str = field.ty.clone().into_token_stream().to_string();

                    //max num check
                    let mut max_num = 0;
                    let attrs_str = data_attr.iter().map(|attr| attr.to_token_stream().to_string()).filter(|attr| attr.contains("maxNum")).collect::<Vec<String>>();
                    if attrs_str.len() != 1 {
                        match data_type_str.as_str() {
                            "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" => panic!("Each field in the struct must have only ONE 'maxNum' attribute"),
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

                    

                    if data_type_str.contains("Vec") {
                        let mut temp = quote! {
                            Vec::
                        };

                        for (i, token) in data_type.clone().into_iter().enumerate() {
                            if i == 0 {
                                continue
                            } else {
                                temp.extend(vec![token]);
                            }
                        }

                        data_type = temp;
                    }

                    let compress_token = match max_num {
                        0 => {
                            quote! {
                                let #data_name = self.#data_name.compress_to_binaries(None)?;
                                all_compressed.push(#data_name);
                                
                            }
                        },
                        _ => {
                            quote! {
                                let #data_name = self.#data_name.compress_to_binaries(Some(#max_num))?;
                                all_compressed.push(#data_name);
                            }
                        }
                    };
                
                    let max_binaries_token = match max_num {
                        0 => {
                            quote! {
                                let #data_name = #data_type::max_binaries(None);
                                size.push(#data_name);
                            }
                        },
                        _ => {
                            quote! {
                                let #data_name = #data_type::max_binaries(Some(#max_num));
                                size.push(#data_name);
                            }
                        }
                    };

                    let decompress_token = quote! {
                        #data_name: a.next().unwrap().decompress(compressed)?
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

            fn compress_to_binaries(self, _max_num: Option<u128>) -> Result<Compressed, CompressError> {
                let mut all_compressed: Vec<Compressed> = vec![];
                #(#compress_tokens)*
        
                let mut binaries = Compressed::Binaries(vec![]);
                for compressed in all_compressed {
                    binaries = binaries.combine(compressed);
                }
                
                Ok(binaries)
            }

            fn compress(self) -> Result<Compressed, CompressError> {
                let compressed_binaries = self.compress_to_binaries(None)?;
                Ok(Compressed::Bytes(compressed_binaries.to_bytes()))
            }
        
            fn max_binaries(_max_num: Option<u128>) -> BinaryChunk {
                let mut size = vec![];
                #(#max_binaries_tokens)*
                
                BinaryChunk::Nested(size)
            }
        
            fn decompress(compressed: Compressed) -> Result<#ident, DecompressError> {
                let mut compressed = compressed.to_binaries();
                Self::decompress_from_binaries(&mut compressed, None)
            }

            fn decompress_from_binaries(compressed: &mut Vec<u8>, _bit_size: Option<usize>) -> Result<Self, DecompressError> where Self: Sized {
                //turn compressed to binaries
                let binary_chunks = Self::max_binaries(None);
        
                let mut a = vec![];
                if let BinaryChunk::Nested(chunks) = binary_chunks {
                    a = chunks;
                }
                let mut a = a.into_iter();
                
        
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


