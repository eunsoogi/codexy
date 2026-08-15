use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};

#[derive(Serialize)]
#[serde(transparent)]
pub(super) struct RequiredNullable<T>(Option<T>);

impl<'de, T: DeserializeOwned> Deserialize<'de> for RequiredNullable<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            Ok(Self::new(None))
        } else {
            serde_json::from_value(value)
                .map(|value| Self::new(Some(value)))
                .map_err(serde::de::Error::custom)
        }
    }
}

impl<T> RequiredNullable<T> {
    pub(super) fn new(value: Option<T>) -> Self {
        Self(value)
    }

    pub(super) fn as_ref(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub(super) fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl RequiredNullable<String> {
    pub(super) fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

#[derive(Default)]
pub(super) enum OptionalField<T> {
    #[default]
    Absent,
    Present(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for OptionalField<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self::Present)
    }
}

impl<T> OptionalField<T> {
    pub(super) fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub(super) fn into_present(self) -> Option<T> {
        match self {
            Self::Absent => None,
            Self::Present(value) => Some(value),
        }
    }
}
