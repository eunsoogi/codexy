use std::fmt;

use serde::{
    Deserialize,
    de::{Error, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Value};

pub(super) fn parse(text: &str) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let StrictValue(value) = StrictValue::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON value")
    }
    fn visit_bool<E: Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }
    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }
    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(value.into())))
    }
    fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
        serde_json::Number::from_f64(value)
            .map(|value| StrictValue(Value::Number(value)))
            .ok_or_else(|| E::custom("invalid number"))
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value.into())))
    }
    fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }
    fn visit_none<E: Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(StrictValue(value)) = access.next_element()? {
            values.push(value);
        }
        Ok(StrictValue(Value::Array(values)))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom(format!("duplicate JSON key {key}")));
            }
            let StrictValue(value) = access.next_value()?;
            values.insert(key, value);
        }
        Ok(StrictValue(Value::Object(values)))
    }
}
