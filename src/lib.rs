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
        syn::Data::Struct(s) => {
            match s.fields {
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
            }
            
            
            let compress_tokens = compress_tokens.iter();
            let max_binaries_tokens = max_binaries_tokens.iter();
            let decompress_tokens = decompress_tokens.iter();
            let output = quote! {
        
                impl comprez::comprezable::Comprezable for #ident {
                    fn compress_to_binaries(self, _max_num: Option<u128>) -> Result<comprez::Compressed, comprez::error::CompressError> {
                        let mut all_compressed: Vec<comprez::Compressed> = vec![];
                        #(#compress_tokens)*
                
                        let mut binaries = comprez::Compressed::Binaries(vec![]);
                        for compressed in all_compressed {
                            binaries = binaries.combine(compressed);
                        }
                        
                        Ok(binaries)
                    }

                    fn compress(self) -> Result<comprez::Compressed, comprez::error::CompressError>  {
                        let compressed_binaries = self.compress_to_binaries(None)?;
                        Ok(comprez::Compressed::Bytes(compressed_binaries.to_bytes()))
                    }
                
                    fn max_binaries(_max_num: Option<u128>) -> comprez::BinaryChunk {
                        let mut size = vec![];
                        #(#max_binaries_tokens)*
                        
                        comprez::BinaryChunk::Nested(size)
                    }
                
                    fn decompress(compressed: comprez::Compressed) -> Result<#ident, comprez::error::DecompressError> {
                        let mut compressed = compressed.to_binaries();
                        Self::decompress_from_binaries(&mut compressed, None)
                    }

                    fn decompress_from_binaries(compressed: &mut Vec<u8>, _bit_size: Option<usize>) -> Result<Self, comprez::error::DecompressError> where Self: Sized {
                        //turn compressed to binaries
                        let binary_chunks = Self::max_binaries(None);
                
                        let mut a = vec![];
                        if let comprez::BinaryChunk::Nested(chunks) = binary_chunks {
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

        },
        syn::Data::Enum(e) => {
            let mut count = 0usize;
            let a = e.variants;
            for variant in a {
                let data_name = variant.ident.clone().into_token_stream();
                let mut data_type = match variant.fields.iter().next() {
                    Some(field) => {
                        field.ty.clone().into_token_stream()
                    },
                    None => {
                        TokenStream::new().into()
                    }
                };
                let data_type_str = data_type.to_string();
                let data_attr = variant.attrs;


                //max num check
                let mut max_num = 0;
                let attrs_str = data_attr.iter().map(|attr| attr.to_token_stream().to_string()).filter(|attr| attr.contains("maxNum")).collect::<Vec<String>>();
                if attrs_str.len() != 1 {
                    validate_data_type(data_type_str.as_str())
                    
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
                        match data_type.is_empty() {
                            true => {
                                quote! {
                                    Self::#data_name => {
                                        comprez::comprezable::comprez_enum_val::<Self>(None, #count, None)
                                    }
                                }
                            },
                            false => {
                                quote! {
                                    Self::#data_name(n) => {
                                        comprez::comprezable::comprez_enum_val(Some(n), #count, None)
                                    }
                                }
                            }
                        }
                    },
                    _ => {
                        match data_type.is_empty() {
                            true => {
                                quote! {
                                    Self::#data_name => {
                                        comprez::comprezable::comprez_enum_val::<Self>(None, #count, None)
                                    }
                                }
                            },
                            false => {
                                quote! {
                                    Self::#data_name(n) => {
                                        comprez::comprezable::comprez_enum_val(Some(n), #count, Some(#max_num))
                                    }
                                }
                            }
                        }
                    }
                };

                let index = count as u128;
                let decompress_token = match max_num {
                    0 => {
                        match data_type.is_empty() {
                            true => {
                                quote! {
                                    #index => {
                                        Ok(Self::#data_name)
                                    }
                                }
                            },
                            false => {
                                quote! {
                                    #index => {
                                        let bit_size = #data_type::max_binaries(None);
                                        let decompressed: #data_type = bit_size.decompress(compressed)?;
                                        Ok(Self::#data_name(decompressed))
                                    }
                                }
                            }
                        }
                    },
                    _ => {
                        match data_type.is_empty() {
                            true => {
                                quote! {
                                    #index => {
                                        Ok(Self::#data_name)
                                    }
                                }
                            },
                            false => {
                                quote! {
                                    #index => {
                                        let bit_size = #data_type::max_binaries(Some(#max_num));
                                        let decompressed: #data_type = bit_size.decompress(compressed)?;
                                        Ok(Self::#data_name(decompressed))
                                    }
                                }
                            }
                        }
                    }
                };

                compress_tokens.push(compress_token);
                decompress_tokens.push(decompress_token);

                count += 1;
            }


            let compress_tokens = compress_tokens.iter();
            let decompress_tokens = decompress_tokens.iter();
            
            let output = quote! {
                impl comprez::comprezable::Comprezable for #ident {

                    fn compress(self) -> Result<comprez::Compressed, comprez::error::CompressError> {
                        let compressed = self.compress_to_binaries(None)?;
                        Ok(comprez::Compressed::Bytes(compressed.to_bytes()))
                    }

                    fn compress_to_binaries(self, max_num: Option<u128>) ->  Result<comprez::Compressed, comprez::error::CompressError> {
                        match self {
                            #(#compress_tokens), *
                        }
                    }

                    fn max_binaries(max_num: Option<u128>) -> comprez::BinaryChunk {
                        comprez::BinaryChunk::Delimeter
                    }

                    fn decompress(compressed: comprez::Compressed) -> Result<Self, comprez::error::DecompressError> where Self: Sized {
                        let mut compressed = compressed.to_binaries();
                        Self::decompress_from_binaries(&mut compressed, None)
                    }

                    fn decompress_from_binaries(compressed: &mut Vec<u8>, _bit_size: Option<usize>) -> Result<Self, comprez::error::DecompressError> where Self:Sized {
                        let index = comprez::comprezable::calc_delimeter_size(compressed, 4)?;
                        match index {
                            #(#decompress_tokens), *
                            _ => {
                                panic!("index out of bound when decompressing enum")
                            }
                        }
                    }
                }
            };


            output.into()
        }
        _ => {
            panic!()
        },
    }

}



fn validate_data_type(data_type: &str) {
    match data_type.contains("Vec") {
        true => {
            let start = match data_type.find('<') {
                Some(n) => n,
                None => panic!("Vector data type must be formatted in: Vec<data>")
            };
            let end = match data_type.find('>') {
                Some(n) => n,
                None => panic!("Vector data type must be formatted in: Vec<data>")
            };
            let data = &data_type[start + 1 .. end];
            validate_primitive_data_type(data.trim());
        },
        false => {
            validate_primitive_data_type(data_type);
        }
    }
}

fn validate_primitive_data_type(data_type: &str) {
    match data_type {
        "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" => {
            panic!("All integer type fields (except Vec<u8>), must have maxNum trait on it, refer to example in doc")
        },
        _ => (),
    }
}