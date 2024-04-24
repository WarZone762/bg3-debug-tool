#![feature(iter_map_windows)]

extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2 as pm2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_macro_input, DeriveInput, Expr, ExprLit, Lit};

#[proc_macro_derive(GameObject, attributes(skip, default_shown))]
pub fn game_object(item: TokenStream) -> TokenStream {
    let DeriveInput { attrs, vis, ident, generics, data } = parse_macro_input!(item);

    let data = match data {
        syn::Data::Struct(x) => x,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    let mut fields = Vec::new();
    for field in data.fields {
        if field.attrs.iter().any(|x| {
            x.meta
                .require_path_only()
                .is_ok_and(|x| x.segments.to_token_stream().to_string() == "skip")
        }) {
            continue;
        }
        let default_shown = field.attrs.iter().any(|x| {
            x.meta
                .require_path_only()
                .is_ok_and(|x| x.segments.to_token_stream().to_string() == "default_shown")
        });
        let name = field.ident.unwrap();
        fields.push((name, default_shown));
    }

    let mut columns = Vec::new();
    let mut draw = Vec::new();
    let mut search_str = Vec::new();
    let mut compare = Vec::new();

    for (i, (field, shown)) in fields.iter().enumerate() {
        let mut new_name = String::new();
        for word in field.to_string().split('_') {
            let mut chars = word.chars();
            new_name.extend(chars.next().map(|x| x.to_ascii_uppercase()));
            new_name.extend(chars);
        }

        columns.push(quote!(crate::menu::search::TableColumn::new(#new_name, #shown, true)));
        draw.push(quote!(#i => crate::menu::search::TableValue::draw(&self.#field, ui)));
        search_str.push(quote!(#i => crate::menu::search::TableValue::search_str(&self.#field)));
        compare.push(
            quote!(#i => crate::menu::search::TableValue::compare(&self.#field, &other.#field)),
        );
    }

    let output = quote! {
        impl crate::menu::search::TableItem for &#ident {
            fn columns() -> Box<[crate::menu::search::TableColumn]> {
                Box::new([
                    #(#columns,)*
                ])
            }

            fn draw(&self, ui: &imgui::Ui, i: usize) {
                match i {
                    #(#draw,)*
                    _ => unreachable!(),
                }
            }

            fn search_str(&self, i: usize) -> String {
                match i {
                    #(#search_str,)*
                    _ => unreachable!(),
                }
            }

            fn compare(&self, other: &Self, i: usize) -> std::cmp::Ordering {
                match i {
                    #(#compare,)*
                    _ => unreachable!(),
                }
            }
        }

    };

    output.into()
}
