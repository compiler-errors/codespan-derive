use std::collections::{BTreeSet, HashMap};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    spanned::Spanned, Attribute, Error, Ident, Index, Lit, Meta, MetaNameValue, Result, Type,
};
use synstructure::{decl_derive, Structure};

decl_derive!([IntoDiagnostic, attributes(file_id, message, note, span)] => diagnostic_derive);

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FieldName {
    Named(String),
    Numbered(u32),
}

fn diagnostic_derive(s: Structure) -> Result<TokenStream> {
    let file_id_attr = syn::parse_str("file_id")?;
    let message_attr = syn::parse_str("message")?;
    let note_attr = syn::parse_str("note")?;
    let span_attr = syn::parse_str("span")?;

    let struct_span = s.ast().span();

    let mut file_id = None;

    for attr in &s.ast().attrs {
        if attr.path == file_id_attr {
            if let Some((_, other_span)) = &file_id {
                let mut err = Error::new(*other_span, "Duplicated #[file_id = ...] attribute");
                err.combine(Error::new(attr.span(), "Second occurrence is here"));
                return Err(err);
            }

            file_id = Some((attr_to_type(attr)?, attr.span()));
        } else if attr.path == message_attr || attr.path == note_attr || attr.path == span_attr {
            return Err(Error::new(
                attr.span(),
                format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
            ));
        }
    }

    let file_id = file_id
        .ok_or_else(|| Error::new(struct_span, "Expected `#[file_id = \"Type\"]` attribute"))?
        .0;

    let mut branches = vec![];

    for v in s.variants() {
        let members = match &v.ast().fields {
            syn::Fields::Unit => HashMap::new(),
            syn::Fields::Named(f) => f
                .named
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    (
                        FieldName::Named(field.ident.as_ref().unwrap().to_string()),
                        format_ident!("__binding_{}", i),
                    )
                })
                .collect(),
            syn::Fields::Unnamed(f) => f
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    (
                        FieldName::Numbered(i as u32),
                        format_ident!("__binding_{}", i),
                    )
                })
                .collect(),
        };

        // TokenStream of the `format!` generated, plus Span of occurrence of
        // attribute in case it's duplicated and we need to error out.
        let mut why = None;
        // Vector of Label creations.
        let mut labels = vec![];
        // Vector of TokenStream of `format!` generated for notes.
        let mut notes = vec![];

        for attr in v.ast().attrs.iter() {
            if attr.path == message_attr {
                if let Some((_, other_span)) = &why {
                    let mut err = Error::new(*other_span, "Duplicated #[message = ...] attribute");
                    err.combine(Error::new(attr.span(), "Second occurrence is here"));
                    return Err(err);
                }

                why = Some((attr_to_format(attr, &members)?, attr.span()));
            } else if attr.path == note_attr {
                let note = attr_to_format(attr, &members)?;
                notes.push(note);
            } else if attr.path == span_attr || attr.path == file_id_attr {
                return Err(Error::new(
                    attr.span(),
                    format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
                ));
            }
        }

        for b in v.bindings() {
            let binding = &b.binding;

            for attr in &b.ast().attrs {
                if attr.path == span_attr {
                    let label = match attr.parse_meta()? {
                        Meta::Path(_) => {
                            quote! {
                                ::codespan_derive::IntoLabel::into_label( #binding )
                            }
                        }
                        Meta::NameValue(MetaNameValue { .. }) => {
                            let message = attr_to_format(&attr, &members)?;

                            quote! {
                                ::codespan_derive::IntoLabel::into_label( #binding )
                                    .with_message( #message )
                            }
                        }
                        _ => return Err(Error::new(attr.span(),
                        format!("Expected `span` attribute to be of the form: `#[span]` or `#[span = \"Message...\"]`"))),
                    };

                    labels.push(label);
                } else if attr.path == message_attr
                    || attr.path == note_attr
                    || attr.path == file_id_attr
                {
                    return Err(Error::new(
                        attr.span(),
                        format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
                    ));
                }
            }
        }

        let why = why
            .ok_or_else(|| {
                Error::new(
                    v.ast().ident.span(),
                    "Expected `#[message = \"Message...\"]` attribute",
                )
            })?
            .0;

        let pat = v.pat();

        branches.push(quote! {
            #pat => {
                ::codespan_derive::Diagnostic::< #file_id >::error()
                    .with_message( #why )
                    .with_labels(vec![ #(#labels),* ])
                    .with_notes(vec![ #(#notes),* ])
            }
        });
    }

    Ok(s.gen_impl(quote! {
        gen impl ::codespan_derive::IntoDiagnostic for @Self {
            type FileId = #file_id ;

            #[allow(dead_code)]
            fn into_diagnostic(&self) -> ::codespan_derive::Diagnostic::< #file_id > {
                match self {
                    #(#branches),*
                    _ => { panic!("Uninhabited type cannot be turned into a Diagnostic") }
                }
            }
        }
    }))
}

fn attr_to_type(attr: &Attribute) -> Result<Type> {
    match attr.parse_meta()? {
        Meta::NameValue(MetaNameValue {
            lit: Lit::Str(ty), ..
        }) => {
            let ty = ty.value();

            if let Ok(ty) = syn::parse_str::<Type>(&ty) {
                Ok(ty)
            } else {
                Err(Error::new(
                    attr.span(),
                    format!("Could not parse valid Rust type from `{}`", ty),
                ))
            }
        }
        _ => Err(Error::new(
            attr.span(),
            format!("Expected `file_id` attribute to be of the form: `#[file_id = \"Type\"]`"),
        )),
    }
}

/// Turns an `#[... = "format string"]` into a `format!()` invocation
fn attr_to_format(attr: &Attribute, members: &HashMap<FieldName, Ident>) -> Result<TokenStream> {
    match attr.parse_meta()? {
        Meta::NameValue(MetaNameValue {
            lit: Lit::Str(msg), ..
        }) => {
            let msg_span = msg.span();
            let mut msg: &str = &msg.value();

            let mut idents = BTreeSet::new();
            let mut out = String::new();

            while !msg.is_empty() {
                if let Some(i) = msg.find('{') {
                    out += &msg[..i];
                    msg = &msg[i + 1..];

                    if &msg[0..2] == "{{" {
                        out += "{{";
                    } else if let Some(j) = msg.find('}') {
                        let (field, rest) = if let Some(k) = msg[0..j].find(":") {
                            (&msg[0..k], Some(&msg[k..j]))
                        } else {
                            (&msg[0..j], None)
                        };

                        // Now reset msg
                        msg = &msg[j + 1..];

                        let member = if let Ok(ident) = syn::parse_str::<Ident>(field) {
                            FieldName::Named(ident.to_string())
                        } else if let Ok(num) = syn::parse_str::<Index>(field) {
                            FieldName::Numbered(num.index)
                        } else {
                            return Err(Error::new(
                                msg_span,
                                format!(
                                    "Expected either a struct member name or index, got `{}`",
                                    field
                                ),
                            ));
                        };

                        out += "{";

                        if let Some(ident) = members.get(&member) {
                            out += &ident.to_string();
                            idents.insert(ident.clone());
                        } else {
                            return Err(Error::new(
                                msg_span,
                                format!(
                                    "Struct member name or index `{}` is not a valid field",
                                    field
                                ),
                            ));
                        }

                        if let Some(rest) = rest {
                            out += rest;
                        }

                        out += "}";
                    } else {
                        return Err(Error::new(msg_span, "Unterminated `{` in format string"));
                    }
                } else {
                    out += msg;
                    msg = "";
                }
            }

            Ok(quote! {
                format!(#out, #(#idents = #idents),*)
            })
        }
        _ => Err(Error::new(
            attr.span(),
            format!(
                "Expected {name} attribute to be of the form: `#[{name} = \"FormatString\"]`",
                name = attr.path.to_token_stream()
            ),
        )),
    }
}
