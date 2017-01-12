//! Used to take a HashMap<String, String> containing field:value pairs of a struct
//! and using it to create a new struct containing that data.

#![recursion_limit = "128"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Ident, VariantData};

#[proc_macro_derive(FromHashmap)]
pub fn from_hashmap(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    // Parse the string representation into a syntax tree
    let ast = syn::parse_macro_input(&source).unwrap();

    // create a vector containing the names of all fields on the struct
    let idents: Vec<Ident> = match ast.body {
        syn::Body::Struct(vdata) => {
            match vdata {
                VariantData::Struct(fields) => {
                    let mut idents = Vec::new();
                    for ref field in fields.iter() {
                        match &field.ident {
                            &Some(ref ident) => idents.push(ident.clone()),
                            &None => panic!("Your struct is missing a field identity!"),
                        }
                    }
                    idents
                },
                VariantData::Tuple(_) | VariantData::Unit => {
                    panic!("You can only derive this for normal structs!");
                },
            }
        },
        syn::Body::Enum(_) => panic!("You can only derive this on structs!"),
    };

    // contains quoted strings containing the struct fields in the same order as
    // the vector of idents.
    let mut keys = Vec::new();
    for ident in idents.iter() {
        keys.push(String::from(ident.as_ref()));
    }

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = quote! {
        /// Attempts to convert the given &str into a T, panicing if it's not successful
        fn parse_pair<T>(v: &str) -> T where T : ::std::str::FromStr {
            let res = v.parse::<T>();
            match res {
                Ok(val) => val,
                Err(_) => panic!(format!("Unable to convert given input into required type: {}", v)),
            }
        }

        impl #impl_generics FromHashmap<#name> for #name #ty_generics #where_clause {
            fn from_hashmap(mut hm: ::std::collections::HashMap<String, String>) -> #name {
                // start with the default implementation
                let mut settings = #name::default();
                #(
                    match hm.entry(String::from(#keys)) {
                        ::std::collections::hash_map::Entry::Occupied(occ_ent) => {
                            // set the corresponding struct field to the value in
                            // the corresponding hashmap if it contains it
                            settings.#idents = parse_pair(occ_ent.get().as_str());
                        },
                        ::std::collections::hash_map::Entry::Vacant(_) => (),
                    }
                )*

                // return the modified struct
                settings
            }
        }
    };

    tokens.parse().unwrap()
}
