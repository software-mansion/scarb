use serde::{Deserialize, Serialize};

/// Merge two `toml::Value` serializable structs.
pub fn toml_merge<'de, T, S>(target: &T, source: &S) -> anyhow::Result<T>
where
    T: Serialize + Deserialize<'de>,
    S: Serialize + Deserialize<'de>,
{
    let mut params = toml::Value::try_from(target)?;
    let source = toml::Value::try_from(source)?;

    params.as_table_mut().unwrap().extend(
        source
            .as_table()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    Ok(toml::Value::try_into(params)?)
}
