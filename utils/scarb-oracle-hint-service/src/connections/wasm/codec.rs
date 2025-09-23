use anyhow::{Result, bail, ensure};
use starknet_core::codec::{Decode, Encode, Error as CodecError};
use starknet_core::types::{ByteArray, Felt};
use starknet_core::utils::{cairo_short_string_to_felt, parse_cairo_short_string};
use wasmtime::component::Type as WasmType;
use wasmtime::component::Val;

#[cfg(test)]
#[path = "codec_tests.rs"]
mod tests;

macro_rules! unsupported {
    ($name:literal) => {
        bail!(concat!("unsupported type: ", $name))
    };
}

/// A subset of [`Type`] that only represents encodable types and also doesn't refer to
/// the WASM instance internally.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ty {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    // Float32,
    // Float64,
    Char,
    String,
    List(Box<Ty>),
    // Record(Record),
    Tuple(Vec<Ty>),
    // Variant(Variant),
    // Enum(Enum),
    Option(Box<Ty>),
    Result(Option<Box<Ty>>, Option<Box<Ty>>),
    // Flags(Flags),
    // Own(ResourceType),
    // Borrow(ResourceType),
    // Future(FutureType),
    // Stream(StreamType),
    // ErrorContext,
}

impl TryFrom<WasmType> for Ty {
    type Error = anyhow::Error;

    fn try_from(ty: WasmType) -> Result<Self, Self::Error> {
        Ok(match ty {
            WasmType::Bool => Self::Bool,
            WasmType::S8 => Self::S8,
            WasmType::U8 => Self::U8,
            WasmType::S16 => Self::S16,
            WasmType::U16 => Self::U16,
            WasmType::S32 => Self::S32,
            WasmType::U32 => Self::U32,
            WasmType::S64 => Self::S64,
            WasmType::U64 => Self::U64,
            WasmType::Float32 => unsupported!("float32"),
            WasmType::Float64 => unsupported!("float64"),
            WasmType::Char => Self::Char,
            WasmType::String => Self::String,
            WasmType::List(list) => Self::List(Box::new(Ty::try_from(list.ty())?)),
            WasmType::Record(_) => unsupported!("record"),
            WasmType::Tuple(tuple) => {
                Self::Tuple(tuple.types().map(Self::try_from).collect::<Result<_>>()?)
            }
            WasmType::Variant(_) => unsupported!("variant"),
            WasmType::Enum(_) => unsupported!("enum"),
            WasmType::Option(option) => Self::Option(Box::new(Ty::try_from(option.ty())?)),
            WasmType::Result(result) => Self::Result(
                result.ok().map(Self::try_from).transpose()?.map(Box::new),
                result.err().map(Self::try_from).transpose()?.map(Box::new),
            ),
            WasmType::Flags(_) => unsupported!("flags"),
            WasmType::Own(_) => unsupported!("own"),
            WasmType::Borrow(_) => unsupported!("borrow"),
            WasmType::Future(_) => unsupported!("future"),
            WasmType::Stream(_) => unsupported!("stream"),
            WasmType::ErrorContext => unsupported!("error-context"),
        })
    }
}

pub fn encode_to_cairo(vals: &[Val]) -> Result<Vec<Felt>> {
    let mut out = Vec::with_capacity(vals.len());

    fn visit(val: &Val, out: &mut Vec<Felt>) -> Result<()> {
        match val {
            Val::Bool(b) => b.encode(out)?,
            Val::S8(s) => Felt::from(*s).encode(out)?,
            Val::U8(u) => u.encode(out)?,
            Val::S16(s) => Felt::from(*s).encode(out)?,
            Val::U16(u) => u.encode(out)?,
            Val::S32(s) => Felt::from(*s).encode(out)?,
            Val::U32(u) => u.encode(out)?,
            Val::S64(s) => Felt::from(*s).encode(out)?,
            Val::U64(u) => u.encode(out)?,

            Val::Float32(_) => unsupported!("float32"),
            Val::Float64(_) => unsupported!("float64"),

            Val::Char(c) => {
                let mut buf = [0u8; 4];
                cairo_short_string_to_felt(c.encode_utf8(&mut buf))?.encode(out)?;
            }

            Val::String(s) => {
                ByteArray::from(s.as_str()).encode(out)?;
            }

            Val::List(vals) => {
                Felt::from(vals.len()).encode(out)?;
                for val in vals {
                    visit(val, out)?;
                }
            }

            Val::Record(_) => unsupported!("record"),

            Val::Tuple(vals) => {
                for val in vals {
                    visit(val, out)?;
                }
            }

            Val::Variant(_, _) => unsupported!("variant"),
            Val::Enum(_) => unsupported!("enum"),

            Val::Option(option) => match option {
                None => Felt::ZERO.encode(out)?,
                Some(v) => {
                    Felt::ONE.encode(out)?;
                    visit(v, out)?
                }
            },

            Val::Result(result) => match result {
                Ok(v) => {
                    Felt::ZERO.encode(out)?;
                    if let Some(v) = v {
                        visit(v, out)?;
                    }
                }
                Err(v) => {
                    Felt::ONE.encode(out)?;
                    if let Some(v) = v {
                        visit(v, out)?;
                    }
                }
            },

            Val::Flags(_) => unsupported!("flags"),
            Val::Resource(_) => unsupported!("own/borrow"),
            Val::Future(_) => unsupported!("future"),
            Val::Stream(_) => unsupported!("stream"),
            Val::ErrorContext(_) => unsupported!("error-context"),
        }
        Ok(())
    }

    for val in vals {
        visit(val, &mut out)?;
    }

    Ok(out)
}

pub fn decode_from_cairo(types: &[Ty], felts: &[Felt]) -> Result<Vec<Val>> {
    let mut felts = felts.iter();
    let mut out = Vec::with_capacity(types.len());

    fn visit<'a>(
        ty: &Ty,
        felts: &mut impl Iterator<Item = &'a Felt>,
        out: &mut Vec<Val>,
    ) -> Result<()> {
        match ty {
            Ty::Bool => out.push(Val::Bool(bool::decode_iter(felts)?)),
            Ty::S8 => out.push(Val::S8(Felt::decode_iter(felts)?.try_into()?)),
            Ty::U8 => out.push(Val::U8(u8::decode_iter(felts)?)),
            Ty::S16 => out.push(Val::S16(Felt::decode_iter(felts)?.try_into()?)),
            Ty::U16 => out.push(Val::U16(u16::decode_iter(felts)?)),
            Ty::S32 => out.push(Val::S32(Felt::decode_iter(felts)?.try_into()?)),
            Ty::U32 => out.push(Val::U32(u32::decode_iter(felts)?)),
            Ty::S64 => out.push(Val::S64(Felt::decode_iter(felts)?.try_into()?)),
            Ty::U64 => out.push(Val::U64(u64::decode_iter(felts)?)),

            Ty::Char => {
                let felt = Felt::decode_iter(felts)?;
                let s = parse_cairo_short_string(&felt)?;
                let mut chars = s.chars();
                let ch = match (chars.next(), chars.next()) {
                    (None, _) => '\0',
                    (Some(c), None) => c,
                    (Some(_), Some(_)) => bail!("expected single-char short string for Char"),
                };
                out.push(Val::Char(ch));
            }

            Ty::String => out.push(Val::String(ByteArray::decode_iter(felts)?.try_into()?)),

            Ty::List(list) => {
                let len = Felt::decode_iter(felts)?.try_into()?;
                let mut items = Vec::with_capacity(len);
                for _ in 0..len {
                    visit(list, felts, &mut items)?;
                }
                out.push(Val::List(items));
            }

            Ty::Tuple(tuple) => {
                let mut items = Vec::with_capacity(tuple.len());
                for ty in tuple {
                    visit(ty, felts, &mut items)?
                }
                out.push(Val::Tuple(items));
            }

            Ty::Option(option) => match Felt::decode_iter(felts)? {
                t if t == Felt::ZERO => out.push(Val::Option(None)),
                t if t == Felt::ONE => {
                    let inner = visit_box(option, felts)?;
                    out.push(Val::Option(Some(inner)));
                }
                tag => Err(CodecError::unknown_enum_tag(tag, "Option<T>"))?,
            },

            Ty::Result(ok, err) => match Felt::decode_iter(felts)? {
                t if t == Felt::ZERO => {
                    out.push(Val::Result(Ok(match ok {
                        Some(ty) => Some(visit_box(ty, felts)?),
                        None => None,
                    })));
                }
                t if t == Felt::ONE => {
                    out.push(Val::Result(Err(match err {
                        Some(ty) => Some(visit_box(ty, felts)?),
                        None => None,
                    })));
                }
                tag => Err(CodecError::unknown_enum_tag(tag, "Result<T, E>"))?,
            },
        }
        Ok(())
    }

    fn visit_box<'a>(ty: &Ty, felts: &mut impl Iterator<Item = &'a Felt>) -> Result<Box<Val>> {
        let mut inner = Vec::with_capacity(1);
        visit(ty, felts, &mut inner)?;
        assert_eq!(
            inner.len(),
            1,
            "inner decode was expected to produce a single value"
        );
        Ok(Box::new(inner.pop().unwrap()))
    }

    for ty in types {
        visit(ty, &mut felts, &mut out)?;
    }

    // Ensure no calldata is left over.
    ensure!(felts.next().is_none(), "not all calldata was consumed");

    Ok(out)
}
