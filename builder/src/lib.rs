use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

struct FieldAttribute {
    name: String,
    value: String,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let parsed_input = parse_macro_input!(input as DeriveInput);

    let struct_name = parsed_input.ident;
    let struct_vis = parsed_input.vis;
    let fields: Vec<syn::Field> = match parsed_input.data {
        syn::Data::Struct(data) => match data.fields {
            syn::Fields::Named(n) => n.named.iter().map(|f| f.clone()).collect(),
            _ => unimplemented!(),
        },
        _ => unimplemented!(),
    };

    // make builder name
    let builder_name = format_ident!("{}Builder", struct_name);

    // grab the visibility, identifier name, and type of all struct fields
    let field_name_types: Vec<(syn::Visibility, syn::Ident, syn::Type)> = fields
        .iter()
        .map(|field| {
            (
                field.vis.clone(),
                field.ident.clone().unwrap(),
                field.ty.clone(),
            )
        })
        .collect();

    // builder struct fields
    let struct_fields =
        field_name_types
            .iter()
            .map(|(vis, ident, ty)| match get_optional_type(ty) {
                Some(_) => quote! {
                    #vis #ident: #ty
                },
                None => quote! {
                    #vis #ident: Option<#ty>
                },
            });

    // declares builder struct
    let builder_struct = quote! {
        #struct_vis struct #builder_name {
            #(#struct_fields),*
        }
    };

    // defaults all fields in the builder to `None`
    let init_builder_fields = field_name_types.iter().map(|(_, ident, _)| {
        quote! {
            #ident: None
        }
    });

    // generates setter methods
    let setters = field_name_types.iter().map(|(vis, ident, ty)| {
        let optional = get_optional_type(ty);
        match optional {
            Some(inner_ty) => quote! {
                #vis fn #ident(&mut self, arg: #inner_ty) -> &mut Self {
                    self.#ident = Some(arg);
                    self
                }
            },
            None => quote! {
                #vis fn #ident(&mut self, arg: #ty) -> &mut Self {
                    self.#ident = Some(arg);
                    self
                }
            },
        }
    });

    let build_method_setters = field_name_types.iter().map(|(_, ident, ty)| {
        let ident_str = ident.to_string();

        match get_optional_type(ty) {
            Some(_) => quote! {
                #ident: self.#ident.clone(),
            },
            None => quote! {
                #ident: match &self.#ident {
                    Some(v) => v.clone(),
                    None => return Err(format!("missing field {}", #ident_str).into()),
                }
            },
        }
    });

    let expanded = quote! {
        #builder_struct

        impl #struct_name {
            #struct_vis fn builder() -> #builder_name {
                #builder_name {
                    #(#init_builder_fields),*
                }
            }
        }

        impl #builder_name {
            pub fn build(&mut self) -> Result<#struct_name, Box<dyn std::error::Error + 'static>> {
                Ok(#struct_name {
                    #(#build_method_setters),*
                })
            }

            #(#setters)*
        }
    };

    eprintln!("expanded: {}", expanded);

    TokenStream::from(expanded)
}

fn get_generic_type(ty: &syn::Type) -> Option<(&syn::Ident, &syn::Type)> {
    match ty {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path:
                syn::Path {
                    leading_colon: None,
                    segments,
                },
        }) => match segments.iter().collect::<Vec<_>>().as_slice() {
            [segment] => match &segment.arguments {
                syn::PathArguments::AngleBracketed(args) => {
                    match args.args.iter().collect::<Vec<_>>().as_slice() {
                        [syn::GenericArgument::Type(ty)] => Some((&segment.ident, ty)),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn get_optional_type(ty: &syn::Type) -> Option<&syn::Type> {
    get_generic_type(ty).and_then(|(ident, ty)| match ident.to_string().as_str() {
        "Option" => Some(ty),
        _ => None,
    })
}
