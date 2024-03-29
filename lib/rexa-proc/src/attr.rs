use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::{
    meta::ParseNestedMeta,
    parse::{discouraged::Speculative, Parse},
    punctuated::Punctuated,
    Ident, LitBool, Token,
};

pub(crate) trait ParseNestedMetaExt {
    fn value_or<T: Parse>(&self, default: T) -> syn::Result<T>;
    fn value_or_else<T: Parse>(&self, default: impl FnOnce() -> T) -> syn::Result<T>;
}

impl<'m> ParseNestedMetaExt for ParseNestedMeta<'m> {
    fn value_or<T: Parse>(&self, default: T) -> syn::Result<T> {
        match self.value() {
            Ok(value) => value.parse(),
            Err(_) => Ok(default),
        }
    }

    fn value_or_else<T: Parse>(&self, default: impl FnOnce() -> T) -> syn::Result<T> {
        match self.value() {
            Ok(value) => value.parse(),
            Err(_) => Ok(default()),
        }
    }
}

pub(crate) struct AttrFlag {
    pub(crate) ident: Ident,
    pub(crate) eq: Option<Token![=]>,
    pub(crate) value: LitBool,
}

impl Parse for AttrFlag {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        let (eq, value) = {
            if let Ok(eq) = input.parse::<Token![=]>() {
                (Some(eq), input.parse()?)
            } else {
                (None, LitBool::new(true, ident.span()))
            }
        };
        Ok(Self { ident, eq, value })
    }
}

impl ToTokens for AttrFlag {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        if let Some(eq) = &self.eq {
            eq.to_tokens(tokens);
            self.value.to_tokens(tokens);
        }
    }
}

impl From<AttrFlag> for LitBool {
    fn from(value: AttrFlag) -> Self {
        value.value
    }
}

pub(crate) struct AttrOptionSet {
    pub(crate) ident: Ident,
    pub(crate) options: HashMap<String, AttrOption>,
}

impl Parse for AttrOptionSet {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            options: {
                let inner;
                let _ = syn::parenthesized!(inner in input);
                let mut opts = HashMap::new();
                for opt in Punctuated::<AttrOption, Token![,]>::parse_terminated(&inner)? {
                    opts.insert(opt.ident().to_string(), opt);
                }
                opts
            },
        })
    }
}

impl AttrOptionSet {
    // pub(crate) fn remove_implicit<T: TryFrom<AttrOption, Error = syn::Error> + From<Span>>(
    //     &mut self,
    //     key: &str,
    // ) -> syn::Result<T> {
    //     let span = self.ident.span();
    //     self.options
    //         .remove(key)
    //         .map(TryFrom::try_from)
    //         .unwrap_or_else(|| Ok(T::from(span)))
    // }

    #[allow(dead_code)]
    pub(crate) fn remove<T: TryFrom<AttrOption, Error = syn::Error>>(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        match self.options.remove(key) {
            Some(v) => v.try_into(),
            None => error!(&self.ident => "missing {key}"),
        }
    }

    // pub(crate) fn try_remove_or<T: TryFrom<AttrOption, Error = syn::Error>>(
    //     &mut self,
    //     key: &str,
    //     or: impl FnOnce() -> syn::Result<T>,
    // ) -> syn::Result<T> {
    //     self.options
    //         .remove(key)
    //         .map(TryFrom::try_from)
    //         .unwrap_or_else(or)
    // }

    // pub(crate) fn remove_flag_or(&mut self, key: &str, default: bool) -> syn::Result<LitBool> {
    //     let span = self.ident.span();
    //     self.try_remove_or(key, || Ok(LitBool::new(default, span)))
    // }

    #[allow(dead_code)]
    pub(crate) fn remove_set<T: TryFrom<AttrOptionSet, Error = syn::Error>>(
        &mut self,
        key: &str,
    ) -> syn::Result<T> {
        match self.options.remove(key) {
            Some(AttrOption::Set(set)) => set.try_into(),
            Some(AttrOption::Flag(flag)) => error!(flag => "expected arguments"),
            None => error!(&self.ident => "missing {key}"),
        }
    }

    // pub(crate) fn remove_implicit_set<
    //     T: TryFrom<AttrOptionSet, Error = syn::Error> + From<Span>,
    // >(
    //     &mut self,
    //     key: &str,
    // ) -> syn::Result<T> {
    //     match self.options.remove(key) {
    //         Some(AttrOption::Set(set)) => set.try_into(),
    //         Some(AttrOption::Flag(flag)) => error!(flag => "expected arguments"),
    //         None => Ok(T::from(self.ident.span())),
    //     }
    // }

    // pub(crate) fn into_unrecognized_err(self) -> syn::Result<()> {
    //     #[warn(clippy::never_loop)]
    //     for opt in self.options.values() {
    //         error!(opt => "unrecognized attribute option");
    //     }
    //     Ok(())
    // }

    // pub(crate) fn process(attr: &Attribute) -> syn::Result<Self> {
    //     let ident = attr.path().require_ident()?.clone();
    //     let mut options = HashMap::new();
    //     for opt in attr.parse_args_with(Punctuated::<AttrOption, Token![,]>::parse_terminated)? {
    //         options.insert(opt.ident().to_string(), opt);
    //     }
    //     Ok(Self { ident, options })
    // }
}

impl ToTokens for AttrOptionSet {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        // FIX :: parentheses...?
        tokens.append_separated(self.options.values(), Token![,](self.ident.span()));
    }
}

pub(crate) enum AttrOption {
    Flag(AttrFlag),
    Set(AttrOptionSet),
}

impl TryFrom<AttrOption> for LitBool {
    type Error = syn::Error;

    fn try_from(value: AttrOption) -> Result<Self, Self::Error> {
        match value {
            AttrOption::Flag(flag) => Ok(flag.value),
            AttrOption::Set(set) => error!(set => "unexpected attribute option arguments"),
        }
    }
}

impl AttrOption {
    fn ident(&self) -> &Ident {
        match self {
            AttrOption::Flag(flag) => &flag.ident,
            AttrOption::Set(set) => &set.ident,
        }
    }
}

impl ToTokens for AttrOption {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            AttrOption::Flag(flag) => flag.to_tokens(tokens),
            AttrOption::Set(set) => set.to_tokens(tokens),
        }
    }
}

impl Parse for AttrOption {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        // TODO :: is there a better way to do this...?
        let fork = input.fork();
        if let Ok(set) = fork.parse() {
            input.advance_to(&fork);
            Ok(Self::Set(set))
        } else {
            input.parse().map(Self::Flag)
        }
    }
}

pub(crate) struct AttrProperty<Right> {
    pub(crate) ident: Ident,
    pub(crate) eq_token: Token![=],
    pub(crate) right: Right,
}

impl<Right: Parse> Parse for AttrProperty<Right> {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            eq_token: input.parse()?,
            right: input.parse()?,
        })
    }
}

impl<Right: ToTokens> ToTokens for AttrProperty<Right> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident.to_tokens(tokens);
        self.eq_token.to_tokens(tokens);
        self.right.to_tokens(tokens);
    }
}
