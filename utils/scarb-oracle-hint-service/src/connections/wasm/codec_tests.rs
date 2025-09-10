use super::*;
use itertools::Itertools;

#[test]
fn roundtrip() {
    fn type_val_pairs<T>(
        ty: Ty,
        new_val: impl Fn(T) -> Val,
        samples: impl IntoIterator<Item = T>,
    ) -> impl Iterator<Item = Vec<(Ty, Val)>> {
        samples
            .into_iter()
            .map(move |sample| vec![(ty.clone(), new_val(sample))])
    }

    let samples: Vec<Vec<(Ty, Val)>> = type_val_pairs(Ty::Bool, Val::Bool, [false, true])
        .chain(type_val_pairs(Ty::S8, Val::S8, [0, i8::MIN, i8::MAX]))
        .chain(type_val_pairs(Ty::U8, Val::U8, [0, u8::MIN, u8::MAX]))
        .chain(type_val_pairs(Ty::S16, Val::S16, [0, i16::MIN, i16::MAX]))
        .chain(type_val_pairs(Ty::U16, Val::U16, [0, u16::MIN, u16::MAX]))
        .chain(type_val_pairs(Ty::S32, Val::S32, [0, i32::MIN, i32::MAX]))
        .chain(type_val_pairs(Ty::U32, Val::U32, [0, u32::MIN, u32::MAX]))
        .chain(type_val_pairs(Ty::S64, Val::S64, [0, i64::MIN, i64::MAX]))
        .chain(type_val_pairs(Ty::U64, Val::U64, [0, u64::MIN, u64::MAX]))
        .chain(type_val_pairs(Ty::Char, Val::Char, ['x', '\0']))
        .chain(type_val_pairs(
            Ty::String,
            Val::String,
            ["Hello 世界! 🌍 Café naïve résumé 北京 🚀".into()],
        ))
        .chain(type_val_pairs(
            Ty::List(Box::new(Ty::String)),
            Val::List,
            [
                vec![],
                vec![Val::String("Hello".into()), Val::String("World".into())],
            ],
        ))
        .chain(type_val_pairs(
            Ty::Tuple(vec![Ty::Bool, Ty::S32, Ty::String]),
            Val::Tuple,
            [vec![
                Val::Bool(false),
                Val::S32(42),
                Val::String("foo".into()),
            ]],
        ))
        .chain(type_val_pairs(Ty::Tuple(vec![]), Val::Tuple, [vec![]]))
        .chain(type_val_pairs(
            Ty::Option(Box::new(Ty::Bool)),
            Val::Option,
            [None, Some(Box::new(Val::Bool(true)))],
        ))
        .chain(type_val_pairs(
            Ty::Result(Some(Box::new(Ty::Bool)), Some(Box::new(Ty::String))),
            Val::Result,
            [
                Ok(Some(Box::new(Val::Bool(true)))),
                Err(Some(Box::new(Val::String("bar".into())))),
            ],
        ))
        .chain(type_val_pairs(
            Ty::Result(None, None),
            Val::Result,
            [Ok(None), Err(None)],
        ))
        .collect();

    // Collect all possible pairs of samples to check for any issues when decoding adjacent values.
    let pairs: Vec<Vec<(Ty, Val)>> = samples
        .iter()
        .cloned()
        .permutations(2)
        .map(|chunk| chunk.iter().flatten().cloned().collect::<Vec<_>>())
        .collect();

    for params in samples.into_iter().chain(pairs) {
        let context = format!("{params:?}");

        let (types, vals): (Vec<Ty>, Vec<Val>) = params.into_iter().unzip();

        let felts =
            encode_to_cairo(&vals).unwrap_or_else(|_| panic!("failed to decode: {context}"));
        let new_vals = decode_from_cairo(&types, &felts)
            .unwrap_or_else(|_| panic!("failed to encode: {context}"));
        assert_eq!(vals, new_vals, "codec roundtrip failed for: {context}");
    }
}
