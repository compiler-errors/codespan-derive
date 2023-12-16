use std::collections::{BTreeSet, HashMap};

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    spanned::Spanned, Attribute, Error, Ident, Index, Lit, Meta, MetaNameValue, Path, Result, Type,
};
use synstructure::{decl_derive, Structure};

decl_derive!([IntoDiagnostic, attributes(file_id, severity, message, render, note, primary, secondary)] => diagnostic_derive);

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FieldName {
    Named(Ident),
    Numbered(u32),
}

fn diagnostic_derive(s: Structure) -> Result<TokenStream> {
    let file_id_attr = syn::parse_str("file_id")?;
    let severity_attr = syn::parse_str("severity")?;
    let message_attr = syn::parse_str("message")?;
    let render_attr = syn::parse_str("render")?;
    let note_attr = syn::parse_str("note")?;
    let primary_attr = syn::parse_str("primary")?;
    let secondary_attr = syn::parse_str("secondary")?;
    let primary_style: Path = syn::parse_str("Primary")?;
    let secondary_style: Path = syn::parse_str("Secondary")?;

    let struct_span = s.ast().span();

    let mut file_id = None;
    let mut severity = None;

    for attr in &s.ast().attrs {
        if attr.path == file_id_attr {
            if let Some((_, other_span)) = &file_id {
                let mut err = Error::new(*other_span, "Duplicated #[file_id(...)] attribute");
                err.combine(Error::new(attr.span(), "Second occurrence is here"));
                return Err(err);
            }

            file_id = Some((attr.parse_args::<Type>()?, attr.span()));
        } else if attr.path == severity_attr {
            if let Some((_, other_span)) = &severity {
                let mut err = Error::new(*other_span, "Duplicated #[severity(...)] attribute");
                err.combine(Error::new(attr.span(), "Second occurrence is here"));
                return Err(err);
            }

            severity = Some((attr.parse_args::<Ident>()?, attr.span()));
        } else if attr.path == message_attr
            || attr.path == render_attr
            || attr.path == note_attr
            || attr.path == primary_attr
            || attr.path == secondary_attr
        {
            return Err(Error::new(
                attr.span(),
                format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
            ));
        }
    }

    let file_id = file_id
        .ok_or_else(|| Error::new(struct_span, "Expected `#[file_id(Type)]` attribute"))?
        .0;
    let severity = severity
        .ok_or_else(|| Error::new(struct_span, "Expected `#[severity(Ident)]` attribute"))?
        .0;

    let mut branches = vec![];

    for v in s.variants() {
        // Create a mapping from field name to pattern binding name
        // Binding names (according to synstructure) are always `__binding_#`
        let members = match &v.ast().fields {
            syn::Fields::Unit => HashMap::new(),
            syn::Fields::Named(f) => f
                .named
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    (
                        FieldName::Named(field.ident.as_ref().unwrap().clone()),
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

        let members_ordered: Vec<_> = (0..members.len())
            .map(|i| format_ident!("__binding_{}", i))
            .collect();

        // TokenStream of the `format!` error message, plus Span of occurrence of
        // attribute in case it's duplicated and we need to error out.
        let mut why = None;
        // TokenStream of the render function call, plus Span of occurrence of
        // attribute in case it's duplicated and we need to error out.
        let mut render = None;
        // Vector of Label creations, corresponding to `#[span]`.
        let mut labels = vec![];
        // Vector of TokenStream of `format!` generated for `#[note]`.
        let mut notes = vec![];

        for attr in v.ast().attrs.iter() {
            if attr.path == message_attr {
                if let Some((_, other_span)) = &why {
                    let mut err = Error::new(*other_span, "Duplicated #[message = ...] attribute");
                    err.combine(Error::new(attr.span(), "Second occurrence is here"));
                    return Err(err);
                } else if let Some((_, other_span)) = &render {
                    let mut err = Error::new(
                        *other_span,
                        "#[message = ...] attribute not compatible with #[render(...)] attribute",
                    );
                    err.combine(Error::new(
                        attr.span(),
                        "#[render(...)] attribute occurs here",
                    ));
                    return Err(err);
                }

                why = Some((attr_to_format(attr, &members)?, attr.span()));
            } else if attr.path == render_attr {
                if let Some((_, other_span)) = &why {
                    let mut err = Error::new(
                        *other_span,
                        "#[message = ...] attribute not compatible with #[render(...)] attribute",
                    );
                    err.combine(Error::new(
                        attr.span(),
                        "#[message = ...] attribute occurs here",
                    ));
                    return Err(err);
                } else if let Some((_, other_span)) = &render {
                    let mut err = Error::new(*other_span, "Duplicated #[render(...)] attribute");
                    err.combine(Error::new(attr.span(), "Second occurrence is here"));
                    return Err(err);
                }

                render = Some((attr_to_render(attr, &members_ordered)?, attr.span()));
            } else if attr.path == note_attr {
                let note = attr_to_format(attr, &members)?;
                notes.push(note);
            } else if attr.path == primary_attr
                || attr.path == secondary_attr
                || attr.path == file_id_attr
                || attr.path == severity_attr
            {
                return Err(Error::new(
                    attr.span(),
                    format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
                ));
            }
        }

        for b in v.bindings() {
            let binding = &b.binding;

            for attr in &b.ast().attrs {
                if attr.path == primary_attr || attr.path == secondary_attr {
                    let style = if attr.path == primary_attr {
                        &primary_style
                    } else {
                        &secondary_style
                    };

                    let label = match attr.parse_meta()? {
                        Meta::Path(_) => {
                            quote! {
                                ::codespan_derive::IntoLabels::into_labels( #binding, ::codespan_derive::LabelStyle::#style )
                            }
                        }
                        Meta::NameValue(MetaNameValue { .. }) => {
                            let message = attr_to_format(&attr, &members)?;

                            quote! {
                                ::codespan_derive::IntoLabels::into_labels( #binding, ::codespan_derive::LabelStyle::#style )
                                    .into_iter()
                                    .map(|x| x.with_message( #message ))
                                    .collect()
                            }
                        }
                        _ => return Err(Error::new(attr.span(),
                        format!("Expected `span` attribute to be of the form: `#[span]` or `#[span = \"Message...\"]`"))),
                    };

                    labels.push(label);
                } else if attr.path == message_attr
                    || attr.path == render_attr
                    || attr.path == note_attr
                    || attr.path == file_id_attr
                    || attr.path == severity_attr
                {
                    return Err(Error::new(
                        attr.span(),
                        format!("Unexpected attribute `{}`", attr.path.to_token_stream()),
                    ));
                }
            }
        }

        let pat = v.pat();

        if let Some((why, _)) = why {
            branches.push(quote! {
                #pat => {
                    ::codespan_derive::Diagnostic::new(::codespan_derive::Severity::#severity)
                        .with_message( #why )
                        #(.with_labels(#labels))*
                        .with_notes(vec![ #(#notes),* ])
                }
            });
        } else if let Some((render, _)) = render {
            branches.push(quote! {
                #pat => {
                    #render
                        #(.with_labels(#labels))*
                        .with_notes(vec![ #(#notes),* ])
                }
            });
        } else {
            return Err(Error::new(
                v.ast().ident.span(),
                "Expected `#[message = \"Message...\"]` or `#[render(function)]` attribute",
            ));
        }
    }

    Ok(s.gen_impl(quote! {
        gen impl ::codespan_derive::IntoDiagnostic for @Self {
            type FileId<'a> = #file_id ;

            #[allow(dead_code)]
            fn into_diagnostic(&self) -> ::codespan_derive::Diagnostic::<Self::FileId<'_>> {
                match self {
                    #(#branches),*
                    _ => { panic!("Uninhabited type cannot be turned into a Diagnostic") }
                }
            }
        }
    }))
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
                if let Some(i) = msg.find(&['{', '}'][..]) {
                    out += &msg[..i];

                    if msg[i..].starts_with("{{") {
                        out += "{{";
                        msg = &msg[i + 2..];
                    } else if msg[i..].starts_with("}}") {
                        out += "}}";
                        msg = &msg[i + 2..];
                    } else if msg[i..].starts_with('}') {
                        return Err(Error::new(msg_span, "Unterminated `}` in format string"));
                    } else {
                        msg = &msg[i + 1..];

                        if let Some(j) = msg.find('}') {
                            let (field, rest) = if let Some(k) = msg[0..j].find(":") {
                                (&msg[0..k], Some(&msg[k..j]))
                            } else {
                                (&msg[0..j], None)
                            };

                            // Now reset msg
                            msg = &msg[j + 1..];

                            let member = if let Ok(ident) = syn::parse_str::<Ident>(field) {
                                FieldName::Named(ident)
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

fn attr_to_render(attr: &Attribute, members: &Vec<Ident>) -> Result<TokenStream> {
    let path: Path = attr.parse_args()?;

    Ok(quote! {
        #path (#(#members),*)
    })
}
