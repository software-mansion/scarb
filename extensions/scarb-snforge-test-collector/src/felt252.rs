use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Felt252(WrapperInner);

impl Felt252 {
    pub fn new(felt: starknet_types_core::felt::Felt) -> Self {
        Self(WrapperInner(felt))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct WrapperInner(starknet_types_core::felt::Felt);

impl Serialize for Felt252 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sub_map: HashMap<_, _> = [("val", self.0.clone())].into_iter().collect();

        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("value", &sub_map)?;
        map.end()
    }
}

// this is copy-pasted BigUint (old felt implementation) inner serialization with inlining
impl Serialize for WrapperInner {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut data: Vec<_> = self
            .0
            .to_bytes_le()
            .chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .rev()
                    .fold(0, |acc, &c| (acc << 8) | u64::from(c))
            })
            .collect();

        normalize(&mut data);

        if let Some((&last, data)) = data.split_last() {
            let last_lo = last as u32;
            let last_hi = (last >> 32) as u32;
            let u32_len = data.len() * 2 + 1 + (last_hi != 0) as usize;
            let mut seq = serializer.serialize_seq(Some(u32_len))?;
            for &x in data {
                seq.serialize_element(&(x as u32))?;
                seq.serialize_element(&((x >> 32) as u32))?;
            }
            seq.serialize_element(&last_lo)?;
            if last_hi != 0 {
                seq.serialize_element(&last_hi)?;
            }
            seq.end()
        } else {
            let data: &[u32] = &[];
            data.serialize(serializer)
        }
    }
}

fn normalize(data: &mut Vec<u64>) {
    if let Some(&0) = data.last() {
        let len = data.iter().rposition(|&d| d != 0).map_or(0, |i| i + 1);
        data.truncate(len);
    }
    if data.len() < data.capacity() / 4 {
        data.shrink_to_fit();
    }
}
