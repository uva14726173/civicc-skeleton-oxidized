use proc_macro::TokenStream;
use quote::{quote, format_ident};
use syn::*;

#[proc_macro_attribute]
pub fn generate_traversal(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut module = parse_macro_input!(item as ItemMod);
    let (_, items) = module.content.as_mut().expect("Module must be inline");

    let mut nodes = Vec::new();

    // Collect all struct + enum names
    for item in items.iter() {
        match item {
            Item::Struct(s) => nodes.push(s.ident.clone()),
            Item::Enum(e) => nodes.push(e.ident.clone()),
            _ => {}
        }
    }

    // Generate Traversal implemenations for the structs + enums
    let mut trav_impls = Vec::new();
    for item in items.iter() {
        match item {
            Item::Struct(s) => trav_impls.push(impl_struct_trav(s, &nodes)),
            Item::Enum(e) => trav_impls.push(impl_enum_trav(e, &nodes)),
            _ => {}
        }
    }

    {// Generate Node enum, Traversal trait, and basic impl
        let mut variants = Vec::new();
        for ident in &nodes {
            variants.push(quote! { #ident(&'a mut #ident) });
        }
        items.push(syn::parse2(quote! {
            pub enum Node<'a> {
                #(#variants),*
            }
        }).unwrap());
        items.push(syn::parse2(quote! {
            pub trait Traversal {
                fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool;
                fn traversal_all<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) {
                    self.traversal(&mut |n| {
                        f(n);
                        true
                    });
                }
                fn traversal_ref<F>(&mut self, f: &F) where F: Fn(Node<'_>) -> bool + ?Sized {
                    self.traversal(&mut |n| f(n));
                }
                fn traversal_refrec<F>(&mut self, f: &F) where F: Fn(Node<'_>, &dyn Fn(Node<'_>) -> bool) -> bool {
                    struct RecHelper<'s> { rf: &'s dyn Fn(&RecHelper, Node<'_>) -> bool }
                    let r1 = RecHelper {
                        rf: &|rh, n| f(n, &|n| (rh.rf)(rh, n))
                    };
                    self.traversal_ref(&|n| (r1.rf)(&r1, n));
                }
            }
        }).unwrap());
        items.push(syn::parse2(quote! {
            impl<T> Traversal for Option<T> where T: Traversal {
                fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                    if let Some(e) = self {
                        e.traversal(f);
                    }
                }
            }
        }).unwrap());
        items.push(syn::parse2(quote! {
            impl<T> Traversal for Box<T> where T: Traversal {
                fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                    self.as_mut().traversal(f);
                }
            }
        }).unwrap());
        items.push(syn::parse2(quote! {
            impl<T> Traversal for Vec<T> where T: Traversal {
                fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                    for e in self {
                        e.traversal(f);
                    }
                }
            }
        }).unwrap());
        for t in ["VecDeque", "LinkedList"] {
            let t = Ident::new(t, proc_macro2::Span::mixed_site());
            items.push(syn::parse2(quote! {
                impl<T> Traversal for std::collections::#t<T> where T: Traversal {
                    fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                        for e in self {
                            e.traversal(f);
                        }
                    }
                }
            }).unwrap());
        }
        for t in ["HashMap", "BTreeMap"] {
            let t = Ident::new(t, proc_macro2::Span::mixed_site());
            items.push(syn::parse2(quote! {
                impl<K,T> Traversal for std::collections::#t<K,T> where T: Traversal {
                    fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                        for e in self.values_mut() {
                            e.traversal(f);
                        }
                    }
                }
            }).unwrap());
        }
    }

    // Push Traversal impls
    for ts in trav_impls {
        let item: syn::Item = syn::parse2(ts).unwrap();
        items.push(item);
    }

    quote!(#module).into()
}

fn is_travable_type(ty: &Type, nodes: &[Ident]) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if path.segments.len() == 1 {
            if let Some(seg) = path.segments.last() {
                if nodes.contains(&seg.ident) {
                    return true;
                }
            }
        }
        if let Some(seg) = path.segments.last() {
            if ["Box", "Option", "Vec", "VecDeque", "LinkedList"].iter().any(|n| seg.ident == n) {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return is_travable_type(inner_ty, nodes);
                    }
                }
            }
            if ["HashMap", "BTreeMap"].iter().any(|n| seg.ident == n) {
                if let PathArguments::AngleBracketed(args) = &seg.arguments {
                    if args.args.len() >= 2 {
                        if let GenericArgument::Type(inner_ty) = &args.args[1] {
                            return is_travable_type(inner_ty, nodes);
                        }
                    }
                }
            }
        }
    }
    false
}

fn impl_struct_trav(
    s: &ItemStruct,
    nodes: &[Ident],
) -> proc_macro2::TokenStream {
    let name = &s.ident;

    let mut trav_statements = Vec::new();
    match &s.fields {
        Fields::Named(fs) => for field in &fs.named {
            if is_travable_type(&field.ty, nodes) {
                let field_name = field.ident.as_ref().unwrap();
                trav_statements.push(quote!{ self.#field_name.traversal(f); });
            }
        }
        Fields::Unnamed(fs) => for (i, field) in fs.unnamed.iter().enumerate() {
            if is_travable_type(&field.ty, nodes) {
                let field_index = Index::from(i);
                trav_statements.push(quote!{ self.#field_index.traversal(f); });
            }
        }
        Fields::Unit => {},
    }

    quote! {
        impl Traversal for #name {
            fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                let auto_trav = f(Node::#name(self));
                if auto_trav {
                    #(#trav_statements)*
                }
            }
        }
    }
}

fn impl_enum_trav(
    e: &ItemEnum,
    nodes: &[Ident],
) -> proc_macro2::TokenStream {
    let name = &e.ident;

    let mut match_arms = Vec::new();
    for variant in &e.variants {
        let variant_name = &variant.ident;
        match &variant.fields {
            Fields::Named(fs) => {
                let field_names: Vec<_> = fs.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                let travs: Vec<_> = fs.named.iter()
                    .filter(|f| is_travable_type(&f.ty, nodes))
                    .map(|f| {
                        let fname = &f.ident;
                        quote! { #fname.traversal(f); }
                    })
                    .collect();
                match_arms.push(quote! {
                    #name::#variant_name { #(#field_names),* } => { #(#travs)* }
                });
            }
            Fields::Unnamed(fs) => {
                let indices: Vec<Ident> = (0..fs.unnamed.len()).map(|i| format_ident!("v{}", i)).collect();
                let travs: Vec<_> = fs.unnamed.iter()
                    .enumerate()
                    .filter(|(_, f)| is_travable_type(&f.ty, nodes))
                    .map(|(i, _)| {
                        let idx = &indices[i];
                        quote! { #idx.traversal(f); }
                    })
                    .collect();
                match_arms.push(quote! {
                    #name::#variant_name(#(#indices),*) => { #(#travs)* }
                });
            }
            Fields::Unit => match_arms.push(quote!{ #name::#variant_name => {} }),
        }
    }

    quote! {
        impl Traversal for #name {
            fn traversal<F>(&mut self, f: &mut F) where F: FnMut(Node<'_>) -> bool {
                let auto_trav = f(Node::#name(self));
                if auto_trav {
                    match self {
                        #(#match_arms),*
                    }
                }
            }
        }
    }
}
