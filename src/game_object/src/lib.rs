#![feature(iter_map_windows)]

extern crate proc_macro;
use pm2::{Ident, Span};
use proc_macro::TokenStream;
use proc_macro2 as pm2;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, Index, LitStr, Member, Token, Visibility,
};

#[proc_macro_derive(GameObject, attributes(skip, column))]
pub fn game_object(item: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(item);
    let type_name = ident.to_string();

    let data = match data {
        syn::Data::Struct(x) => x,
        syn::Data::Enum(_) => unimplemented!(),
        syn::Data::Union(_) => unimplemented!(),
    };

    let mut fields = Vec::new();
    'outer: for (i, field) in data.fields.into_iter().enumerate() {
        let ident = match field.ident {
            Some(x) => Member::Named(x),
            None => Member::Unnamed(Index { index: i as _, span: Span::call_site() }),
        };
        for attr in &field.attrs {
            if attr.path().is_ident("skip") {
                continue 'outer;
            } else if attr.path().is_ident("column") {
                fields.push((ident, match attr.parse_args::<ColumnDef>() {
                    Ok(x) => x,
                    Err(x) => return x.to_compile_error().into(),
                }));
                continue 'outer;
            }
        }
        if matches!(field.vis, Visibility::Inherited) {
            continue;
        }
        fields.push((ident, ColumnDef::default()));
    }

    let mut columns = Vec::new();

    let mut visit = Vec::new();
    let mut visit_field = Vec::new();
    let mut visit_all = Vec::new();
    let mut visit_parallel = Vec::new();

    let mut tbl_cmp = vec![quote!(std::cmp::Ordering::Equal)];

    let mut debug = vec![quote!(f.debug_struct(#type_name))];
    let mut search_str = vec![quote!(f.debug_struct(""))];

    for (i, (field, column_def)) in fields.into_iter().enumerate() {
        let name = column_def.name.unwrap_or_else(|| {
            let mut name = String::new();
            for word in field.to_token_stream().to_string().split('_') {
                let mut chars = word.chars();
                name.extend(chars.next().map(|x| x.to_ascii_uppercase()));
                name.extend(chars);
            }
            name
        });
        let visible = column_def.visible;
        let getter = |ident: Ident| {
            if let Some(x) = &column_def.getter {
                quote!((#ident.#x()))
            } else {
                quote!((&#ident.#field))
            }
        };
        let get_self = getter(Ident::new("self", Span::call_site()));
        let get_other = getter(Ident::new("other", Span::call_site()));

        columns
            .push(quote!(crate::menu::search::table::TableColumn::new(#name, #visible, #visible)));

        visit.push(quote!(#i => crate::menu::search::table_value::GameObjectVisitor::visit(
            visitor,
            #name,
            #get_self,
        )));
        visit_field.push(
            quote!(#name => crate::menu::search::table_value::GameObjectVisitor::visit(
                visitor,
                #name,
                #get_self,
            )),
        );
        visit_all.push(quote!(crate::menu::search::table_value::GameObjectVisitor::visit(
            &mut visitor,
            #name,
            #get_self,
        )));
        visit_parallel.push(
            quote!(#i => crate::menu::search::table_value::GameObjectParallelVisitor::visit_parallel(
                visitor,
                #name,
                #get_self,
                #get_other,
            )),
        );

        tbl_cmp.push(quote!(.then_with(|| (*#get_self).tbl_cmp(#get_other))));

        debug.push(quote!(.field(&#name, #get_self)));
        search_str.push(quote!(.field_with(&#name, |f| #get_self.search_str(f))));
    }

    let mut output = Vec::new();
    for r#type in [ident.to_token_stream(), quote!(&#ident), quote!(&mut #ident)] {
        output.push(quote! {
            impl crate::menu::search::table::ColumnsTableItem for #r#type {
                fn columns() -> Box<[crate::menu::search::table::TableColumn]> {
                    Box::new([
                        #(#columns,)*
                    ])
                }

                fn visit_parallel<T: crate::menu::search::table_value::GameObjectParallelVisitor>(
                    &self,
                    visitor: &mut T,
                    other: &Self,
                    i: usize
                ) -> T::Return {
                    match i {
                        #(#visit_parallel,)*
                        _ => unreachable!(),
                    }
                }
            }

            impl crate::menu::search::table::TableItem for #r#type {
                fn visit<T: crate::menu::search::table_value::GameObjectVisitor>(
                    &self,
                    visitor: &mut T,
                    i: usize
                ) -> T::Return {
                    match i {
                        #(#visit,)*
                        _ => unreachable!(),
                    }
                }

                fn visit_field<T: crate::menu::search::table_value::GameObjectVisitor>(
                    &self,
                    visitor: &mut T,
                    name: &str,
                ) -> Option<T::Return> {
                    Some(match name {
                        #(#visit_field,)*
                        _ => return None,
                    })
                }

                fn visit_all<T: crate::menu::search::table_value::GameObjectFullVisitor>(
                    &self,
                    mut visitor: T,
                ) -> T::Finish {
                    #(#visit_all;)*
                    visitor.finish()
                }
            }

            impl crate::menu::search::table_value::TableValue for #r#type {
                fn type_name() -> String {
                    #type_name.into()
                }

                fn draw(&self, ui: &imgui::Ui) {
                    crate::menu::search::table::details_view(ui, self);
                }

                fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    #(#search_str)*.finish()
                }

                fn is_container(&self) -> bool {
                    true
                }
            }

            impl crate::menu::search::table_value::TableOrd for #r#type {
                fn tbl_cmp(&self, other: &Self) -> std::cmp::Ordering {
                    #(#tbl_cmp)*
                }
            }
        });
    }

    quote! {
        #(#output)*

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #(#debug)*.finish()
            }
        }
    }
    .into()
}

#[proc_macro_derive(TableValue)]
pub fn table_value(item: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(item);
    let type_name = ident.to_string();

    let data = match data {
        syn::Data::Struct(_) => unimplemented!(),
        syn::Data::Enum(x) => x,
        syn::Data::Union(_) => unimplemented!(),
    };

    let mut variants = Vec::new();
    for variant in data.variants {
        match variant.fields {
            syn::Fields::Unit => variants.push(variant),
            _ => {
                return syn::Error::new_spanned(variant, "only unit types are supported")
                    .to_compile_error()
                    .into()
            }
        }
    }

    let mut draw = Vec::new();
    let mut search_str = Vec::new();

    for variant in variants {
        let name = variant.ident;
        let name_str = name.to_string();

        draw.push(quote!(#ident::#name => ui.text_wrapped(#name_str)));
        search_str.push(quote!(#ident::#name => f.write_str(#name_str)));
    }

    let mut output = Vec::new();
    for (r#type, this) in [
        (ident.to_token_stream(), quote!(self)),
        (quote!(&#ident), quote!(*self)),
        (quote!(&mut #ident), quote!(*self)),
    ] {
        output.push(quote! {
            impl crate::menu::search::table_value::TableValue for #r#type {
                fn type_name() -> String {
                    #type_name.into()
                }

                fn draw(&self, ui: &imgui::Ui) {
                    match #this {
                        #(#draw,)*
                    }
                }

                fn search_str(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match #this {
                        #(#search_str,)*
                    }
                }
            }

            impl crate::menu::search::table_value::TableOrd for #r#type {
                fn tbl_cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.cmp(other)
                }
            }
        });
    }

    quote!(#(#output)*).into()
}

#[derive(Debug, Default)]
struct ColumnDef {
    name: Option<String>,
    visible: bool,
    include_in_search: bool,
    getter: Option<Ident>,
}

impl Parse for ColumnDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut visible = false;
        let mut include_in_search = false;
        let mut getter = None;

        loop {
            let attr_name = input.parse::<Ident>()?;
            match attr_name.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse::<LitStr>()?.value());
                }
                "visible" => visible = true,
                "include_in_search" => include_in_search = true,
                "getter" => {
                    input.parse::<Token![=]>()?;
                    getter = Some(input.parse::<Ident>()?)
                }
                _ => return Err(input.error("unexpected attribute")),
            }
            if input.is_empty() {
                break;
            } else {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { name, visible, include_in_search, getter })
    }
}
