use std::collections::HashMap;

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use syn::{
    parse::{Parse, Parser},
    parse_macro_input, parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    Arm, Attribute, Expr, ImplItemFn, ItemImpl, Path, Signature, Stmt, Token, Type,
};

// WARNING :: got way too "clever" with this one

macro_rules! fmt_line_in {
    () => {
        ::std::concat!("(@ line ", ::std::line!(), " in ", ::std::file!(), ")")
    };
}

macro_rules! todo_str {
    () => {
        ::std::concat!("not yet implemented ", fmt_line_in!())
    };
    ($extra:literal) => {
        ::std::concat!("not yet implemented: ", $extra, " ", fmt_line_in!())
    };
}

#[allow(unused_macro_rules)]
macro_rules! format_error {
    ($span:expr => $($arg:tt)+) => {
        ::syn::parse::Error::new_spanned($span, ::std::format!($($arg)+))
    };
    ($($arg:tt)+) => {
        ::syn::parse::Error::new(::proc_macro2::Span::call_site(), ::std::format!($($arg)+))
    };
}

macro_rules! error {
    ($span:expr => $($arg:tt)+) => {
        return ::std::result::Result::Err(format_error!($span => $($arg)+))
    };
}

#[allow(unused_macros)]
macro_rules! internal_error {
    ($span:expr => $($arg:tt)+) => {
        error!($span => ::std::concat!("internal error ", fmt_line_in!(), ": {}"), ::std::format!($($arg),+))
    }
}

/// Like [`std::todo`], but it expands to return a [`syn::parse::Error`] instead of panic.
#[allow(unused_macro_rules)]
macro_rules! todo {
    () => {
        return ::std::result::Result::Err(format_error!(todo_str!()))
    };
    ($span:expr) => {
        return ::std::result::Result::Err(format_error!($span => todo_str!()))
    };
    ($span:expr => $($arg:tt)+) => {
        return ::std::result::Result::Err(
                format_error!($span => todo_str!("{}"), ::std::format!($($arg)+))
            )
    };
}

#[allow(unused_macro_rules)]
macro_rules! tokens_error {
    ($tokens:expr, $span:expr => $($arg:tt)+) => {
        format_error!($span => $($arg)+).into_compile_error().to_tokens($tokens)
    };
    ($tokens:expr => $($arg:tt)+) => {
        format_error!($($arg)+).into_compile_error().to_tokens($tokens)
    };
}

#[allow(unused_macro_rules)]
macro_rules! tokens_todo {
    ($tokens:expr, $span:expr => $($arg:tt)+) => {
        tokens_error!($tokens, $span => todo_str!("{}"), ::std::format!($($arg)+))
    };
    ($tokens:expr => $($arg:tt)+) => {
        tokens_error!($tokens => todo_str!("{}"), ::std::format!($($arg)+))
    };
    ($tokens:expr, $span:expr) => {
        tokens_error!($tokens, $span => todo_str!())
    };
    ($tokens:expr) => {
        tokens_error!($tokens => todo_str!())
    };
}

mod deliver;
use deliver::*;

mod deliver_only;
use deliver_only::*;

mod input;
use input::*;

mod export;
use export::*;

mod attr;
use attr::*;

struct Metadata {
    rexa: Path,
    syrup: Path,
    futures: Path,
    tracing: Option<Path>,

    object_t: Type,
    session_t: Type,
    item_t: Type,
    error_t: Type,
    deliver_only_result_t: Type,
    deliver_result_t: Type,
    args_t: Type,
    resolver_t: Type,
    from_fn: Path,
}

impl Parse for Metadata {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let args = Punctuated::<AttrProperty<Path>, Token![,]>::parse_terminated(input)?;
        let (rexa, syrup, futures, tracing) = {
            let mut rexa = None;
            let mut syrup = None;
            let mut futures = None;
            let mut tracing = None;
            for arg in args {
                if arg.ident == "rexa" {
                    rexa = Some(arg.right);
                } else if arg.ident == "syrup" {
                    syrup = Some(arg.right);
                } else if arg.ident == "futures" {
                    futures = Some(arg.right);
                } else if arg.ident == "tracing" {
                    tracing = Some(arg.right);
                } else {
                    error!(arg => "unrecognized impl_object property");
                }
            }
            (
                rexa.unwrap_or(parse_quote!(::rexa)),
                syrup.unwrap_or(parse_quote!(::syrup)),
                futures.unwrap_or(parse_quote!(::futures)),
                tracing,
            )
        };

        let item_t: Type = parse_quote!(#syrup::Item);
        // let deliver_error_t: Type = parse_quote!(#rexa::captp::object::DeliverError);
        // let deliver_only_error_t: Type = parse_quote!(#rexa::captp::object::DeliverOnlyError);
        let error_t: Type = parse_quote!(#rexa::captp::object::ObjectError);
        Ok(Self {
            object_t: parse_quote!(#rexa::captp::object::Object),
            session_t: parse_quote!(::std::sync::Arc<dyn #rexa::captp::AbstractCapTpSession + ::std::marker::Send + ::std::marker::Sync>),
            deliver_only_result_t: parse_quote!(::std::result::Result<(), #error_t>),
            deliver_result_t: parse_quote!(::std::result::Result<(), #error_t>),
            args_t: parse_quote!(#syrup::de::Sequence<'args>),
            resolver_t: parse_quote!(#rexa::captp::GenericResolver),
            item_t,
            error_t,
            from_fn: parse_quote!(::std::convert::From::from),

            rexa,
            syrup,
            futures,
            tracing,
        })
    }
}

enum ObjectFn<'context> {
    Deliver(DeliverFn<'context>),
    DeliverOnly(DeliverOnlyFn<'context>),
    Export(ExportFn),
}

impl<'cx> ObjectFn<'cx> {
    fn process(context: &'cx Metadata, f: &mut ImplItemFn) -> Result<Option<Self>, syn::Error> {
        let mut res = None;
        let mut attr_index = None;
        for (i, attr) in f.attrs.iter().enumerate() {
            if attr.path().is_ident("deliver") {
                attr_index = Some(i);
                let (attr, inputs) = DeliverAttr::process(context, attr, &mut f.sig)?;
                res = Some(Self::Deliver(DeliverFn::process(
                    context, attr, &f.sig, inputs,
                )?));
                break;
            } else if attr.path().is_ident("deliver_only") {
                attr_index = Some(i);
                let (attr, inputs) = DeliverOnlyAttr::process(context, attr, &mut f.sig)?;
                res = Some(Self::DeliverOnly(DeliverOnlyFn::process(
                    context, attr, &f.sig, inputs,
                )?));
                break;
            } else if attr.path().is_ident("exported") {
                attr_index = Some(i);
                res = Some(Self::Export(ExportFn::process(&mut f.sig)?));
            }
        }

        if let Some(i) = attr_index {
            f.attrs.remove(i);
        }

        Ok(res)
    }
}

struct ObjectDefParser<'context> {
    context: &'context Metadata,
}

impl<'cx> Parser for ObjectDefParser<'cx> {
    type Output = ObjectDef<'cx>;

    fn parse2(self, tokens: TokenStream) -> syn::Result<Self::Output> {
        use syn::parse2;

        let mut base = parse2::<ItemImpl>(tokens)?;
        let mut deliver_fallback = None;
        let mut deliver_fns = HashMap::new();
        let mut deliver_verbatim = None;
        let mut deliver_only_fallback = None;
        let mut deliver_only_fns = HashMap::new();
        let mut deliver_only_verbatim = None;
        let mut export_fn = None;
        for item in &mut base.items {
            match item {
                syn::ImplItem::Fn(f) => match ObjectFn::process(self.context, f)? {
                    Some(ObjectFn::Export(export)) => {
                        export_fn = Some(export);
                    }
                    Some(ObjectFn::Deliver(del)) => {
                        let DeliverAttr::Normal { fallback, .. } = &del.attr else {
                            deliver_verbatim = Some(del);
                            continue;
                        };
                        if fallback.value {
                            deliver_fallback = Some(del);
                        } else {
                            deliver_fns.insert(del.symbol().value(), del);
                        }
                    }
                    Some(ObjectFn::DeliverOnly(del)) => {
                        let DeliverOnlyAttr::Normal { fallback, .. } = &del.attr else {
                            deliver_only_verbatim = Some(del);
                            continue;
                        };
                        if fallback.value {
                            deliver_only_fallback = Some(del);
                        } else {
                            deliver_only_fns.insert(del.symbol().value(), del);
                        }
                    }
                    None => { /* skip */ }
                },
                syn::ImplItem::Verbatim(tt) => {
                    todo!(&tt => "handle verbatim impl item: {tt:?}")
                }
                syn::ImplItem::Macro(m) => {
                    error!(m => "macro items not supported in #[impl_object] blocks")
                }
                syn::ImplItem::Const(_) | syn::ImplItem::Type(_) => { /* ignore */ }
                item => todo!(&item => "unrecognized impl item: {item:?}"),
            }
        }
        Ok(Self::Output {
            span: base.span(),
            self_ty: (*base.self_ty).clone(),
            base,
            deliver_fns,
            deliver_fallback,
            deliver_verbatim,
            deliver_only_fns,
            deliver_only_fallback,
            deliver_only_verbatim,
            export_fn,
        })
    }
}

struct ObjectDef<'context> {
    span: Span,
    self_ty: Type,

    base: ItemImpl,

    deliver_fns: HashMap<String, DeliverFn<'context>>,
    deliver_fallback: Option<DeliverFn<'context>>,
    deliver_verbatim: Option<DeliverFn<'context>>,

    deliver_only_fns: HashMap<String, DeliverOnlyFn<'context>>,
    deliver_only_fallback: Option<DeliverOnlyFn<'context>>,
    deliver_only_verbatim: Option<DeliverOnlyFn<'context>>,

    export_fn: Option<ExportFn>,
}

#[proc_macro_attribute]
pub fn impl_object(
    attr_input: proc_macro::TokenStream,
    obj_input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let metadata = parse_macro_input!(attr_input as Metadata);
    let Metadata {
        rexa,
        syrup,
        futures,
        tracing,
        object_t,
        session_t,
        item_t,
        error_t,
        args_t,
        resolver_t,
        deliver_only_result_t,
        deliver_result_t,
        from_fn,
    } = &metadata;

    let ObjectDef {
        span,
        self_ty,
        base,
        deliver_fns,
        deliver_only_fns,
        deliver_fallback,
        deliver_only_fallback,
        deliver_verbatim,
        deliver_only_verbatim,
        export_fn,
    } = {
        let parser = ObjectDefParser { context: &metadata };
        parse_macro_input!(obj_input with parser)
    };

    let (impl_generics, _, where_clause) = base.generics.split_for_impl();

    let deliver_only_sig: Signature = parse_quote! {
        fn deliver_only(&self, session: #session_t, mut args: #args_t) -> #deliver_only_result_t
    };

    let deliver_sig: Signature = parse_quote! {
        fn deliver<'result>(&'result self,
            session: #session_t,
            mut args: #args_t,
            resolver: #resolver_t
        ) -> #futures::future::BoxFuture<'result, #deliver_result_t>
    };

    let get_id: Vec<Stmt> = parse_quote! {
        let mut args = ::std::collections::VecDeque::from(args);
        let __id = match args.pop_front() {
            Some(#item_t::Symbol(__id)) => __id,
            Some(__item) => return Err(#error_t::unexpected("Symbol", 0, __item)),
            None => return Err(#error_t::missing(0, "Symbol"))
        };
        let mut args_pos = 1;
    };

    //let deliver_only: ImplItemFn = if let Some(verbatim) = deliver_only_verbatim {
    //    parse_quote! {
    //        #deliver_only_sig {
    //            #verbatim
    //        }
    //    }
    //} else if deliver_only_fns.is_empty() {
    //    if let Some(tracing) = tracing {
    //        parse_quote! {
    //            #deliver_only_sig {
    //                #tracing::warn!(session_key_hash = #rexa::hash(session.remote_vkey()), ?args, "unexpected deliver_only");
    //                Ok(())
    //            }
    //        }
    //    } else {
    //        parse_quote! {
    //            #deliver_only_sig {
    //                Ok(())
    //            }
    //        }
    //    }
    //} else {
    //    let mut deliver_only_arms = deliver_only_fns
    //        .into_iter()
    //        .map(|(symbol, del)| {
    //            parse_quote_spanned! {del.span()=> #symbol => #del }
    //        })
    //        .collect::<Vec<Arm>>();
    //    deliver_only_arms.push(match deliver_only_fallback {
    //        Some(fallback) => parse_quote_spanned! {fallback.span()=> id => #fallback},
    //        _ => parse_quote! { id => todo!("unrecognized deliver_only function: {id}") },
    //    });
    //    parse_quote! {
    //        #deliver_only_sig {
    //            #(#get_id)*
    //            match __id.as_str() {
    //                #(#deliver_only_arms),*
    //            }
    //        }
    //    }
    //};

    //let deliver: ImplItemFn = if let Some(verbatim) = deliver_verbatim {
    //    parse_quote! {
    //        #deliver_sig {
    //            #verbatim
    //        }
    //    }
    //} else if deliver_fns.is_empty() {
    //    let result: Expr = parse_quote! {
    //        #futures::FutureExt::boxed(async move {
    //            resolver.break_promise(&::std::format!("unrecognized delivery: {args:?}")).await.map_err(#from_fn)
    //        })
    //    };
    //    if let Some(tracing) = tracing {
    //        parse_quote! {
    //            #deliver_sig {
    //                #tracing::warn!(session_key_hash = #rexa::hash(session.remote_vkey()), ?args, "unexpected deliver");
    //                #result
    //            }
    //        }
    //    } else {
    //        parse_quote! {
    //            #deliver_sig {
    //                #result
    //            }
    //        }
    //    }
    //} else {
    //    let mut deliver_arms = deliver_fns
    //        .into_iter()
    //        .map(|(symbol, del)| {
    //            parse_quote_spanned! {del.span()=> #symbol => #del }
    //        })
    //        .collect::<Vec<Arm>>();
    //
    //    deliver_arms.push(match deliver_fallback {
    //        Some(fallback) => parse_quote_spanned! {fallback.span()=> id => #fallback},
    //        _ => parse_quote! { id => resolver.break_promise(&::std::format!("unrecognized function: {id}")).await.map_err(#from_fn) },
    //    });
    //    parse_quote! {
    //        #deliver_sig {
    //            use #futures::FutureExt;
    //            async move {
    //                #(#get_id)*
    //                // `let ...` so that we get more helpful errors
    //                let __res: #deliver_result_t = match __id.as_str() {
    //                    #(#deliver_arms),*
    //                };
    //                __res
    //            }.boxed()
    //        }
    //    }
    //};

    let deliver_only: ImplItemFn = parse_quote! {
        #deliver_only_sig {
            ::std::todo!()
        }
    };

    let deliver: ImplItemFn = parse_quote! {
        #deliver_sig {
            ::std::todo!()
        }
    };

    let exported: Option<ImplItemFn> = export_fn.map(|exported| parse_quote! {
            fn exported(&self, remote_key: &#rexa::captp::RemoteKey, position: #rexa::captp::msg::DescExport) {
                #exported
            }
        });

    // let instrument_only: Option<Attribute> = tracing.as_ref().map(|tracing| parse_quote_spanned! {tracing.span()=> #[#tracing::instrument(fields(session = #rexa::hash(&session.remote_vkey()), args = %#syrup::ser::to_pretty(&args).unwrap()))]});

    quote_spanned! {span=>
        #base

        #[automatically_derived]
        impl #impl_generics #object_t for #self_ty #where_clause {
            #deliver_only

            #deliver

            #exported
        }
    }
    .into()
}
