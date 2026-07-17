use thiserror::Error;

pub(crate) const OPAQUE_ID_BYTES: usize = 32;

#[derive(Debug, Error)]
#[error("secure random identifier generation failed")]
pub struct RandomIdError;

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Result<Self, $crate::id::RandomIdError> {
                let mut bytes = [0_u8; $crate::id::OPAQUE_ID_BYTES];
                getrandom::fill(&mut bytes).map_err(|_| $crate::id::RandomIdError)?;
                let value = base64::Engine::encode(
                    &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                    bytes,
                );
                Ok(Self(value))
            }

            pub(crate) fn expose(&self) -> &str {
                &self.0
            }

            pub(crate) fn parse(value: &str) -> Option<Self> {
                let bytes = base64::Engine::decode(
                    &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                    value,
                )
                .ok()?;
                (bytes.len() == $crate::id::OPAQUE_ID_BYTES).then(|| Self(value.to_owned()))
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&"<redacted>")
                    .finish()
            }
        }
    };
}

pub(crate) use opaque_id;
