use syrup::{de::RecordFieldAccess, Deserialize, Serialize};

mod start_session;
pub use start_session::*;

mod abort;
pub use abort::*;

mod import_export {
    use syrup::{
        de::{RecordFieldAccess, Visitor},
        Deserialize, Serialize, Symbol,
    };

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[syrup(name = "desc:export")]
    pub struct DescExport {
        pub position: u64,
    }

    impl From<u64> for DescExport {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    impl std::fmt::Debug for DescExport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[syrup(name = "desc:import-object")]
    pub struct DescImportObject {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportObject {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl From<DescExport> for DescImportObject {
        fn from(position: DescExport) -> Self {
            Self {
                position: position.position,
            }
        }
    }

    #[derive(Clone, Copy, Serialize, Deserialize)]
    #[syrup(name = "desc:import-promise")]
    pub struct DescImportPromise {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportPromise {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl From<u64> for DescImportPromise {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    #[derive(Clone, Copy)]
    pub enum DescImport {
        Object(DescImportObject),
        Promise(DescImportPromise),
    }

    impl Default for DescImport {
        fn default() -> Self {
            Self::Object(DescImportObject { position: 0 })
        }
    }

    impl std::fmt::Debug for DescImport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DescImport::Object(o) => o.fmt(f),
                DescImport::Promise(p) => p.fmt(f),
            }
        }
    }

    impl From<DescImportObject> for DescImport {
        fn from(value: DescImportObject) -> Self {
            Self::Object(value)
        }
    }

    impl From<DescImportPromise> for DescImport {
        fn from(value: DescImportPromise) -> Self {
            Self::Promise(value)
        }
    }

    impl<'de> Deserialize<'de> for DescImport {
        fn deserialize<D: syrup::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            struct __Visitor;
            impl<'de> Visitor<'de> for __Visitor {
                type Value = DescImport;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "either desc:import-object or desc:import-promise")
                }

                fn visit_record<R: syrup::de::RecordAccess<'de>>(
                    self,
                    rec: R,
                ) -> Result<Self::Value, R::Error> {
                    let (mut rec, label) = rec.label::<Symbol<&str>>()?;
                    match label.0 {
                        "desc:import-promise" => Ok(DescImport::Promise(DescImportPromise {
                            position: rec.next_field()?.unwrap(),
                        })),
                        "desc:import-object" => Ok(DescImport::Object(DescImportObject {
                            position: rec.next_field()?.unwrap(),
                        })),
                        _ => todo!(),
                    }
                }
            }
            de.deserialize_record(__Visitor)
        }
    }

    impl Serialize for DescImport {
        fn serialize<Ser: syrup::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
            match self {
                DescImport::Object(o) => o.serialize(s),
                DescImport::Promise(p) => p.serialize(s),
            }
        }
    }
}
pub use import_export::*;

mod deliver {
    use super::{DescExport, DescImport};
    use syrup::{ser::SerializeRecord, Deserialize, RawSyrup, Serialize};

    #[derive(Clone, Copy)]
    pub struct OpDeliverOnlySlice<'args, Arg> {
        pub to_desc: DescExport,
        pub args: &'args [Arg],
    }

    impl<'args, Arg> OpDeliverOnlySlice<'args, Arg> {
        pub fn new(to_desc: DescExport, args: &'args [Arg]) -> Self {
            Self { to_desc, args }
        }
    }

    impl<'arg, Arg> std::fmt::Debug for OpDeliverOnlySlice<'arg, Arg>
    where
        Self: Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<'args, Arg> Serialize for OpDeliverOnlySlice<'args, Arg>
    where
        [Arg]: Serialize,
    {
        fn serialize<Ser: syrup::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
            let mut rec = s.serialize_record("op:deliver-only", Some(2))?;
            rec.serialize_field(&self.to_desc)?;
            rec.serialize_field(self.args)?;
            rec.end()
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "op:deliver-only")]
    pub struct OpDeliverOnly<Arg> {
        pub to_desc: DescExport,
        pub args: Vec<Arg>,
    }

    impl<Arg> std::fmt::Debug for OpDeliverOnly<Arg>
    where
        Self: Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<Arg> OpDeliverOnly<Arg> {
        pub fn new(position: u64, args: Vec<Arg>) -> Self {
            Self {
                to_desc: position.into(),
                args,
            }
        }
    }

    #[derive(Clone, Copy)]
    pub struct OpDeliverSlice<'args, Arg> {
        pub to_desc: DescExport,
        pub args: &'args [Arg],
        pub answer_pos: Option<u64>,
        pub resolve_me_desc: DescImport,
    }

    impl<'args, Arg> OpDeliverSlice<'args, Arg> {
        pub fn new(
            to_desc: DescExport,
            args: &'args [Arg],
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Self {
            Self {
                to_desc,
                args,
                answer_pos,
                resolve_me_desc,
            }
        }
    }

    impl<'arg, Arg> std::fmt::Debug for OpDeliverSlice<'arg, Arg>
    where
        Self: Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<'args, Arg> Serialize for OpDeliverSlice<'args, Arg>
    where
        [Arg]: Serialize,
    {
        fn serialize<Ser: syrup::ser::Serializer>(&self, s: Ser) -> Result<Ser::Ok, Ser::Error> {
            let mut rec = s.serialize_record("op:deliver", Some(4))?;
            rec.serialize_field(&self.to_desc)?;
            rec.serialize_field(self.args)?;
            rec.serialize_field(&self.answer_pos)?;
            rec.serialize_field(&self.resolve_me_desc)?;
            rec.end()
        }
    }

    #[derive(Clone, Serialize, Deserialize)]
    #[syrup(name = "op:deliver")]
    pub struct OpDeliver<Arg> {
        pub to_desc: DescExport,
        pub args: Vec<Arg>,
        pub answer_pos: Option<u64>,
        pub resolve_me_desc: DescImport,
    }

    impl<Arg> std::fmt::Debug for OpDeliver<Arg>
    where
        Self: Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    impl<Arg> OpDeliver<Arg> {
        pub fn new(
            position: u64,
            args: Vec<Arg>,
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Self {
            Self {
                to_desc: position.into(),
                args,
                answer_pos,
                resolve_me_desc,
            }
        }
    }

    impl OpDeliver<syrup::RawSyrup> {
        pub fn from_ident_args<'arg, Arg: Serialize + 'arg>(
            position: u64,
            ident: impl AsRef<str>,
            args: impl IntoIterator<Item = &'arg Arg>,
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Result<Self, syrup::Error<'static>> {
            Ok(Self::new(
                position,
                RawSyrup::vec_from_ident_iter(ident, args.into_iter())?,
                answer_pos,
                resolve_me_desc,
            ))
        }
    }
}
pub use deliver::*;

mod handoff {
    use super::PublicKey;
    use crate::locator::NodeLocator;
    use syrup::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize)]
    #[syrup(name = "desc:handoff-give")]
    pub struct DescHandoffGive {
        pub receiver_key: PublicKey,
        pub exporter_location: NodeLocator,
        #[syrup(with = syrup::bytes::vec)]
        pub session: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub gifter_side: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub gift_id: Vec<u8>,
    }

    impl std::fmt::Debug for DescHandoffGive
    where
        Self: syrup::Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }

    #[derive(Clone, Deserialize, Serialize)]
    #[syrup(name = "desc:handoff-receive")]
    pub struct DescHandoffReceive {
        #[syrup(with = syrup::bytes::vec)]
        pub receiving_session: Vec<u8>,
        #[syrup(with = syrup::bytes::vec)]
        pub receiving_side: Vec<u8>,
        pub handoff_count: u64,
        pub signed_give: DescHandoffGive,
    }

    impl std::fmt::Debug for DescHandoffReceive
    where
        Self: syrup::Serialize,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&syrup::ser::to_pretty(self).unwrap())
        }
    }
}
pub use handoff::*;

/// Used for [`CapTpSession::recv_event`](CapTpSession::recv_event).
#[derive(Clone)]
pub(super) enum Operation<Inner> {
    DeliverOnly(OpDeliverOnly<Inner>),
    Deliver(OpDeliver<Inner>),
    // Pick(OpPick),
    Abort(OpAbort),
    // Listen(OpListen),
    // GcExport(OpGcExport),
    // GcAnswer(OpGcAnswer),
}

impl<Inner> std::fmt::Debug for Operation<Inner>
where
    OpDeliverOnly<Inner>: Serialize,
    OpDeliver<Inner>: Serialize,
    OpAbort: Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeliverOnly(d) => d.fmt(f),
            Self::Deliver(d) => d.fmt(f),
            Self::Abort(a) => a.fmt(f),
        }
    }
}

// TODO :: improve syrup lib's handling of enums
impl<'de, Inner: syrup::Deserialize<'de>> syrup::Deserialize<'de> for Operation<Inner> {
    fn deserialize<D: syrup::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct __Visitor<Inner>(std::marker::PhantomData<Inner>);
        impl<'de, Inner: Deserialize<'de>> syrup::de::Visitor<'de> for __Visitor<Inner> {
            type Value = Operation<Inner>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("op:start-session, op:deliver-only, op:deliver, op:pick, op:abort, op:listen, op:gc-export, or op:gc-answer")
            }

            fn visit_record<R: syrup::de::RecordAccess<'de>>(
                self,
                rec: R,
            ) -> Result<Self::Value, R::Error> {
                let (mut rec, label) = rec.label::<syrup::Symbol<&str>>()?;
                match label.0 {
                    "op:deliver-only" => Ok(Operation::DeliverOnly(OpDeliverOnly {
                        to_desc: rec.next_field()?.unwrap(),
                        args: rec.next_field()?.unwrap(),
                    })),
                    "op:deliver" => Ok(Operation::Deliver(OpDeliver {
                        to_desc: rec.next_field()?.unwrap(),
                        args: rec.next_field()?.unwrap(),
                        answer_pos: rec.next_field()?.unwrap(),
                        resolve_me_desc: rec.next_field()?.unwrap(),
                    })),
                    "op:pick" => todo!("op:pick"),
                    "op:abort" => Ok(Operation::Abort(OpAbort {
                        reason: rec.next_field()?.unwrap(),
                    })),
                    "op:listen" => todo!("op:listen"),
                    "op:gc-export" => todo!("op:gc-export"),
                    "op:gc-answer" => todo!("op:gc-answer"),
                    _ => Err(todo!("unrecognized operation")),
                }
            }
        }
        de.deserialize_record(__Visitor(std::marker::PhantomData))
    }
}
