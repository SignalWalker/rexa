use proc_macro2::{Span, TokenStream};
use quote::{quote_spanned, ToTokens};
use syn::{parse_quote_spanned, spanned::Spanned, Expr, FnArg, LitBool, PatType, Receiver, Type};

use crate::{attr::ParseNestedMetaExt, Metadata};

pub(crate) enum DeliverInput<'cx> {
    Receiver(Receiver),
    Mapped {
        is_resolver: bool,
        map: Expr,
    },
    Syrup {
        ty: Type,
        map: Expr,
        context: &'cx Metadata,
    },
}

impl<'cx> DeliverInput<'cx> {
    fn session(span: Span) -> Self {
        Self::Mapped {
            is_resolver: false,
            map: parse_quote_spanned! {span=> session},
        }
    }

    fn resolver(span: Span) -> Self {
        Self::Mapped {
            is_resolver: true,
            map: parse_quote_spanned! {span=> resolver},
        }
    }

    fn args(span: Span) -> Self {
        Self::Mapped {
            is_resolver: false,
            map: parse_quote_spanned! {span=> args.into()},
        }
    }

    fn syrup_inner(context: &'cx Metadata, span: Span, ty: &Type) -> Expr {
        let error_t = &context.error_t;
        let syrup = &context.syrup;
        parse_quote_spanned! {span=>
            <#ty as #syrup::FromSyrupItem>::from_syrup_item(&arg).map_err(|_| #error_t::unexpected(::std::stringify!(#ty), args_pos - 1, arg))?
        }
    }

    fn syrup(context: &'cx Metadata, span: Span, ty: Type) -> Self {
        Self::Syrup {
            map: Self::syrup_inner(context, span, &ty),
            ty,
            context,
        }
    }

    fn syrup_from(context: &'cx Metadata, span: Span, from: Type, to: Type) -> Self {
        let mut map = Self::syrup_inner(context, span, &from);
        map = parse_quote_spanned! {span=> ::std::convert::Into::<#to>::into(#map)};
        Self::Syrup {
            map,
            ty: from,
            context,
        }
    }

    fn is_resolver(&self) -> bool {
        matches!(
            self,
            Self::Mapped {
                is_resolver: true,
                ..
            }
        )
    }

    fn process(context: &'cx Metadata, input: &mut PatType) -> syn::Result<Self> {
        let mut obj_attr_index = None;
        let mut res = None;
        for (i, attr) in input.attrs.iter().enumerate() {
            if attr.path().is_ident("arg") {
                obj_attr_index = Some(i);
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("session") {
                        res = Some(Self::session(meta.path.span()));
                        Ok(())
                    } else if meta.path.is_ident("args") {
                        res = Some(Self::args(meta.path.span()));
                        Ok(())
                    } else if meta.path.is_ident("resolver") {
                        res = Some(Self::resolver(meta.path.span()));
                        Ok(())
                    } else if meta.path.is_ident("map") {
                        res = Some(Self::Mapped {
                            is_resolver: false,
                            map: meta.value()?.parse()?,
                        });
                        Ok(())
                    } else if meta.path.is_ident("syrup_from") {
                        let from: Type = meta.value()?.parse()?;
                        res = Some(Self::syrup_from(
                            context,
                            meta.path.span(),
                            from,
                            (*input.ty).clone(),
                        ));
                        Ok(())
                    } else if meta.path.is_ident("syrup") {
                        res = Some(Self::Syrup {
                            ty: (*input.ty).clone(),
                            map: meta
                                .value_or_else(|| parse_quote_spanned! {meta.path.span()=> arg })?,
                            context,
                        });
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized arg attribute"))
                    }
                })?;
                break;
            }
        }
        if let Some(i) = obj_attr_index {
            input.attrs.remove(i);
        }

        match res {
            Some(arg) => Ok(arg),
            None => Ok(Self::syrup(context, input.span(), (*input.ty).clone())),
        }
    }
}

impl<'cx> ToTokens for DeliverInput<'cx> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            DeliverInput::Receiver(receiver) => {
                let expr: Expr = parse_quote_spanned! {receiver.span()=> self};
                expr.to_tokens(tokens);
            }
            DeliverInput::Mapped { map, .. } => map.to_tokens(tokens),
            DeliverInput::Syrup {
                ty,
                map,
                context: Metadata { error_t, .. },
            } => {
                quote_spanned! {ty.span()=>
                    match args.pop_front() {
                        Some(arg) => { args_pos += 1; #map },
                        None => return ::std::result::Result::Err(#error_t::missing(args_pos, ::std::stringify!(#ty))), // ::std::todo!(::std::concat!("missing argument: ", #id, ": ", ::std::stringify!(#ty)))
                    }
                }
                .to_tokens(tokens);
            }
        }
    }
}

pub(crate) fn process_inputs<'cx, 'arg>(
    context: &'cx Metadata,
    inputs: impl IntoIterator<Item = &'arg mut FnArg>,
) -> syn::Result<(LitBool, Vec<DeliverInput<'cx>>)> {
    let mut takes_resolver = LitBool::new(false, Span::call_site());
    let mut res = Vec::<DeliverInput<'cx>>::new();
    for input in inputs {
        match input {
            FnArg::Typed(input) => {
                let input = DeliverInput::process(context, input)?;
                if input.is_resolver() {
                    takes_resolver = LitBool::new(true, input.span());
                }
                res.push(input);
            }
            FnArg::Receiver(receiver) => {
                res.push(DeliverInput::Receiver(receiver.clone()));
            }
        }
    }
    Ok((takes_resolver, res))
}
