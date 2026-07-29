use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize,
    de::{Deserializer, Error as _, MapAccess, SeqAccess, Visitor},
};

pub(super) fn unique_object(text: &str, label: &str) -> Vec<String> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    deserializer
        .deserialize_map(UniqueObject)
        .err()
        .map(|error| vec![format!("{label} {error}")])
        .unwrap_or_default()
}

struct UniqueObject;

impl<'de> Visitor<'de> for UniqueObject {
    type Value = ();
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object with unique keys")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        read_map(&mut map)?;
        Ok(())
    }
}

struct UniqueValue;

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueValue)
    }
}

impl<'de> Visitor<'de> for UniqueValue {
    type Value = Self;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON")
    }
    fn visit_bool<E>(self, _: bool) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_i64<E>(self, _: i64) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_u64<E>(self, _: u64) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_f64<E>(self, _: f64) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_str<E>(self, _: &str) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_none<E>(self) -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Ok(self)
    }
    fn visit_seq<A>(self, mut sequence: A) -> Result<Self, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<UniqueValue>()?.is_some() {}
        Ok(self)
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>,
    {
        read_map(&mut map)?;
        Ok(self)
    }
}

fn read_map<'de, A>(map: &mut A) -> Result<(), A::Error>
where
    A: MapAccess<'de>,
{
    let mut keys = BTreeSet::new();
    while let Some(key) = map.next_key::<String>()? {
        if !keys.insert(key.clone()) {
            return Err(A::Error::custom(format!("must not repeat {key}")));
        }
        let _: UniqueValue = map.next_value()?;
    }
    Ok(())
}
