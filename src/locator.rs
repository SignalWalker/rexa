//! - [Draft Specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    num::ParseIntError,
    ops::{Index, IndexMut},
    str::FromStr,
    string::FromUtf8Error,
};

use fluent_uri::{
    component::{Host, Scheme},
    encoding::{
        encoder::{self, Query, RegName, Userinfo},
        EStr, EString,
    },
    Builder, Uri,
};
use syrup::{symbol, Decode, Encode};

#[allow(clippy::doc_markdown)] // false positive on `OCapN`
/// An identifier for a single OCapN node.
///
/// From the [draft specification](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md):
/// > This identifies an OCapN node, not a specific object. This includes enough information to specify which netlayer and provide that netlayer with all of the information needed to create a bidirectional channel to that node.
#[derive(Clone, Decode, Encode, PartialEq, Eq)]
#[syrup(label = "ocapn-node")]
pub struct NodeLocator<'input> {
    /// Distinguishes the target node from other nodes accessible through the netlayer specified by
    /// the transport key.
    pub designator: Cow<'input, str>,
    /// Specifies the netlayer that should be used to access the target node.
    #[syrup(as = syrup::Symbol)]
    pub transport: Cow<'input, str>,
    /// Additional connection information.
    #[syrup(with = syrup::optional_map)]
    pub hints: HashMap<syrup::Symbol<'input>, Cow<'input, str>>,
}

impl<'i> std::fmt::Debug for NodeLocator<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseUriError<Uri> {
    #[error(transparent)]
    Uri(#[from] fluent_uri::error::ParseError<Uri>),
    #[error(transparent)]
    Port(#[from] ParseIntError),
    #[error(transparent)]
    DecodeHint(#[from] FromUtf8Error),
    #[error("expected `ocapn`, found: `{0}`")]
    UnrecognizedScheme(String),
    #[error("no authority component found in parsed uri")]
    MissingAuthority,
    #[error("no transport component found in host str")]
    MissingTransport,
}

impl<'i> TryFrom<Uri<&'i str>> for NodeLocator<'i> {
    type Error = ParseUriError<()>;

    fn try_from(uri: Uri<&'i str>) -> Result<Self, Self::Error> {
        if let Some(scheme) = uri.scheme().map(Scheme::as_str) {
            if !scheme.eq_ignore_ascii_case("ocapn") {
                return Err(ParseUriError::UnrecognizedScheme(scheme.to_owned()));
            }
        }

        let Some(authority) = uri.authority() else {
            return Err(ParseUriError::MissingAuthority);
        };

        let (designator, transport) = {
            let host = authority.host();
            let Some((designator, transport)) = host.rsplit_once('.') else {
                return Err(ParseUriError::MissingTransport);
            };
            (designator, transport)
        };

        let mut hints = HashMap::new();

        if let Some(userinfo) = authority.userinfo() {
            hints.insert(symbol!["userinfo"], userinfo.decode().into_string()?);
        }

        if let Some(port) = authority.port() {
            hints.insert(symbol!["port"], Cow::Borrowed(port));
        }

        if let Some(query) = uri.query() {
            for (key, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
                hints.insert(
                    syrup::Symbol(key.decode().into_string()?),
                    value.decode().into_string()?,
                );
            }
        }

        Ok(Self {
            designator: designator.into(),
            transport: transport.into(),
            hints,
        })
    }
}

//impl<'i> FromStr for NodeLocator<'i> {
//    type Err = ParseUriError<()>;
//
//    fn from_str(s: &str) -> Result<Self, Self::Err> {
//        Self::try_from(Uri::parse(s)?)
//    }
//}

impl<'i> TryFrom<&NodeLocator<'i>> for Uri<String> {
    type Error = fluent_uri::error::BuildError;
    fn try_from(loc: &NodeLocator<'i>) -> Result<Self, Self::Error> {
        loc.build_uri(EStr::new(""))
    }
}

impl<'i> TryFrom<NodeLocator<'i>> for Uri<String> {
    type Error = fluent_uri::error::BuildError;
    fn try_from(loc: NodeLocator<'i>) -> Result<Self, Self::Error> {
        Self::try_from(&loc)
    }
}

impl<'i> std::fmt::Display for NodeLocator<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Uri::try_from(self).unwrap().fmt(f)
    }
}

impl<'i, Q> Index<&Q> for NodeLocator<'i>
where
    Q: Eq + std::hash::Hash + ?Sized,
    syrup::Symbol<'i>: Borrow<Q>,
{
    type Output = Cow<'i, str>;

    fn index(&self, index: &Q) -> &Self::Output {
        &self.hints[index]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AsSocketAddrError {
    #[error(transparent)]
    ParseAddr(#[from] AddrParseError),
    #[error("locator does not contain port hint")]
    MissingPort,
    #[error(transparent)]
    ParsePort(#[from] ParseIntError),
}

impl<'i> NodeLocator<'i> {
    pub fn new(designator: impl Into<Cow<'i, str>>, transport: impl Into<Cow<'i, str>>) -> Self {
        Self {
            designator: designator.into(),
            transport: transport.into(),
            hints: HashMap::new(),
        }
    }

    pub fn encoded_query(&self) -> Option<EString<Query>> {
        if self.hints.is_empty() {
            None
        } else {
            let mut query = EString::<Query>::new();
            for (k, v) in self.hints.iter() {
                if k.0 == "port" || k.0 == "userinfo" {
                    // these are encoded as part of the authority uri component
                    continue;
                }
                if !query.is_empty() {
                    query.push_byte(b'&');
                }
                query.encode::<Query>(k.0.as_bytes());
                query.push_byte(b'=');
                query.encode::<Query>(v.as_bytes());
            }
            if query.is_empty() {
                None
            } else {
                Some(query)
            }
        }
    }

    pub fn encoded_userinfo(&self) -> Option<EString<Userinfo>> {
        self.hint("userinfo").map(|info| {
            let mut estr = EString::<Userinfo>::new();
            estr.encode::<Userinfo>(info.as_bytes());
            estr
        })
    }

    pub fn encoded_host(&self) -> EString<RegName> {
        let mut estr = EString::<RegName>::new();
        estr.encode::<RegName>(&self.designator.as_bytes());
        estr.push_byte(b'.');
        estr.encode::<RegName>(&self.transport.as_bytes());
        estr
    }

    pub fn hint<Q>(&self, key: &Q) -> Option<&Cow<'i, str>>
    where
        syrup::Symbol<'i>: Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        self.hints.get(key)
    }

    pub fn hint_into<V: FromStr, Q>(&self, key: &Q) -> Option<Result<V, V::Err>>
    where
        syrup::Symbol<'i>: Borrow<Q>,
        Q: std::hash::Hash + Eq + ?Sized,
    {
        self.hints.get(key).map(|h| V::from_str(h))
    }

    pub fn ipv6_from_designator(&self) -> Result<Ipv6Addr, AddrParseError> {
        Ipv6Addr::from_str(&self.designator)
    }

    pub fn ipv4_from_designator(&self) -> Result<Ipv4Addr, AddrParseError> {
        Ipv4Addr::from_str(&self.designator)
    }

    pub fn ip_from_designator(&self) -> Result<IpAddr, AddrParseError> {
        IpAddr::from_str(&self.designator)
    }

    pub fn as_socket_addr(&self) -> Result<SocketAddr, AsSocketAddrError> {
        Ok(SocketAddr::new(
            self.ip_from_designator()?,
            self.hint_into("port")
                .ok_or(AsSocketAddrError::MissingPort)??,
        ))
    }

    fn build_uri(
        &self,
        path: &EStr<fluent_uri::encoding::encoder::Path>,
    ) -> Result<Uri<String>, fluent_uri::error::BuildError> {
        Uri::builder()
            .scheme(Scheme::new("ocapn"))
            .authority(|b| {
                let reg_name = self.encoded_host();
                b.optional(Builder::userinfo, self.encoded_userinfo().as_deref())
                    // NOTE :: must use regname here because we can't encode the transport into an
                    // ip host
                    .host(Host::RegName(&reg_name))
                    .optional(Builder::port, self.hint("port").map(Cow::borrow))
            })
            .path(path)
            .optional(Builder::query, self.encoded_query().as_deref())
            .build()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseSturdyRefUriError {
    #[error(transparent)]
    Locator(#[from] ParseUriError<()>),
    #[error("no path component in parsed uri")]
    MissingPath,
    #[error("uri path component does not start with `s/`")]
    InvalidPath,
}

impl From<fluent_uri::error::ParseError> for ParseSturdyRefUriError {
    fn from(value: fluent_uri::error::ParseError) -> Self {
        Self::Locator(ParseUriError::Uri(value))
    }
}

/// A unique identifier for
#[derive(Clone, Decode, Encode)]
#[syrup(label = "ocapn-sturdyref")]
pub struct SturdyRefLocator<'input> {
    pub node_locator: NodeLocator<'input>,
    pub swiss_num: Cow<'input, [u8]>,
}

impl<'i> SturdyRefLocator<'i> {
    pub fn new(node_locator: NodeLocator<'i>, swiss_num: impl Into<Cow<'i, [u8]>>) -> Self {
        Self {
            node_locator,
            swiss_num: swiss_num.into(),
        }
    }

    pub fn encoded_path(&self) -> EString<fluent_uri::encoding::encoder::Path> {
        use fluent_uri::encoding::encoder::Path;
        let mut path = EString::<Path>::new();
        path.push_estr(EStr::new("/s/"));
        path.encode::<Path>(&self.swiss_num);
        path
    }
}

impl<'i> std::fmt::Debug for SturdyRefLocator<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

impl<'i> TryFrom<Uri<&'i str>> for SturdyRefLocator<'i> {
    type Error = ParseSturdyRefUriError;

    fn try_from(uri: Uri<&'i str>) -> Result<Self, Self::Error> {
        const SWISS_PREFIX: &[u8] = b"/s/";

        let node_locator = NodeLocator::try_from(uri)?;

        let path: Cow<'i, [u8]> = uri.path().decode().into_bytes();

        if path.is_empty() {
            return Err(ParseSturdyRefUriError::MissingPath);
        }

        if !path.starts_with(SWISS_PREFIX) {
            return Err(ParseSturdyRefUriError::InvalidPath);
        }

        Ok(Self {
            node_locator,
            swiss_num: Cow::Owned(path[SWISS_PREFIX.len()..].to_owned()),
        })
    }
}

//impl<'i> FromStr for SturdyRefLocator<'i> {
//    type Err = ParseSturdyRefUriError;
//
//    fn from_str(s: &str) -> Result<Self, Self::Err> {
//        Self::try_from(Uri::parse(s)?)
//    }
//}

impl<'i> TryFrom<&SturdyRefLocator<'i>> for Uri<String> {
    type Error = fluent_uri::error::BuildError;
    fn try_from(loc: &SturdyRefLocator<'i>) -> Result<Self, Self::Error> {
        let path = loc.encoded_path();
        loc.node_locator.build_uri(&path)
    }
}

impl<'i> TryFrom<SturdyRefLocator<'i>> for Uri<String> {
    type Error = fluent_uri::error::BuildError;
    fn try_from(loc: SturdyRefLocator<'i>) -> Result<Self, Self::Error> {
        Self::try_from(&loc)
    }
}

impl<'i> std::fmt::Display for SturdyRefLocator<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Uri::try_from(self).unwrap().fmt(f)
    }
}
