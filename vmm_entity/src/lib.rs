use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
    parse::{self, Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    Attribute, Error, Ident, ItemStruct, LitStr, Token,
};

#[derive(Debug)]
struct Args {
    pub vars: Vec<LitStr>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        let vars = Punctuated::<syn::LitStr, Token![,]>::parse_terminated(input)?;

        if vars.len() != 2 {
            return Err(Error::new(
                vars.span(),
                "expected 2 arguments: version, migrator",
            ));
        }

        Ok(Args {
            vars: vars.into_iter().collect::<Vec<LitStr>>(),
        })
    }
}

#[proc_macro_attribute]
pub fn vmm_entity(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(item as ItemStruct);
    let struct_ident = item_struct.ident.clone();

    let args = parse_macro_input!(args as Args);

    let version = &args.vars[0];
    let version_str = LitStr::new(
        format!("\"{}\".into()", version.value()).as_ref(),
        version.span(),
    );
    let version_regex = LitStr::new(format!("^{}$", version.value()).as_ref(), version.span());

    let migrator = &args.vars[1];
    let migrator_ident = Ident::new(&migrator.value(), migrator.span());

    let kind = struct_ident.to_string();
    let kind_str = LitStr::new(format!("\"{}\".into()", kind).as_ref(), kind.span());
    let kind_regex = LitStr::new(format!("^{}$", kind).as_ref(), version.span());

    if let syn::Fields::Named(ref mut fields) = item_struct.fields {
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! {
                    #[validate(pattern=#version_regex)]
                    #[derivative(Default(value=#version_str))]
                    pub api_version: String
                })
                .expect("failed to parse the apiVersion field"),
        );
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! {
                    #[validate(pattern=#kind_regex)]
                    #[derivative(Default(value=#kind_str))]
                    pub kind: String
                })
                .expect("failed to parse the kind field"),
        );
        fields.named.push(
            syn::Field::parse_named
                .parse2(quote! {
                    #[validate]
                    pub metadata: crate::database::metadata::Metadata
                })
                .expect("failed to parse the metadata field"),
        );
    } else {
        return quote_spanned! {
            item_struct.span() =>
            compile_error!("expected a named struct");
        }
        .into();
    }

    let impl_entity = quote! {
        impl crate::database::entity::Entity for #struct_ident {
            const API_VERSION: &'static str = #version;
            const KIND: &'static str = #kind;
            type Type = #struct_ident;

            fn migrator(version: &str) -> Option<fn(serde_json::value::Value)
                -> Result<serde_json::value::Value, crate::database::error::Error>> {
                #migrator_ident (version)
            }
        }
    };

    let attr_derive: Attribute = parse_quote! {
        #[derive(serde::Serialize, serde::Deserialize, Debug, serde_valid::Validate, derivative::Derivative)]
    };

    let attr_default: Attribute = parse_quote! {
        #[derivative(Default)]
    };

    let attr_camel_case: Attribute = parse_quote! {
        #[serde(rename_all = "camelCase")]
    };

    item_struct.attrs.insert(0, attr_derive);
    item_struct.attrs.insert(1, attr_default);
    item_struct.attrs.insert(2, attr_camel_case);

    quote! {
        #item_struct
        #impl_entity
    }
    .into()
}

#[proc_macro_attribute]
pub fn vmm_entity_struct(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_struct = parse_macro_input!(item as ItemStruct);
    let attr_derive: Attribute = parse_quote! {
        #[derive(serde::Serialize, serde::Deserialize, Debug, serde_valid::Validate, Default)]
    };

    let attr_camel_case: Attribute = parse_quote! {
        #[serde(rename_all = "camelCase")]
    };

    item_struct.attrs.insert(0, attr_derive);
    item_struct.attrs.insert(1, attr_camel_case);

    quote! {
        #item_struct
    }
    .into()
}
