pub mod cluster;
pub mod peer;
pub mod topology;
pub mod util;
pub mod vpn;
pub mod cleo;

use std::marker::PhantomData;

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
#[error("Could not convert from `{from}` to `{to}`: {details}")]
pub struct ConversionError {
    from: &'static str,
    to: &'static str,
    details: String,
}

impl ConversionError {
    pub fn new<From, To>(details: impl Into<String>) -> Self {
        Self {
            from: std::any::type_name::<From>(),
            to: std::any::type_name::<To>(),
            details: details.into(),
        }
    }
}

pub struct ConversionErrorBuilder<From, To> {
    _from: PhantomData<From>,
    _to: PhantomData<To>,
}

#[allow(clippy::new_ret_no_self)]
impl<From, To> ConversionErrorBuilder<From, To> {
    pub fn message(details: impl Into<String>) -> ConversionError {
        ConversionError::new::<From, To>(details)
    }
    pub fn field_not_set(field: impl Into<String>) -> ConversionError {
        let details = format!("Field '{}' not set", field.into());
        ConversionError::new::<From, To>(details)
    }
}

#[macro_export]
macro_rules! conversion {
    (
        type Model = $Model:ty;
        type Proto = $Proto:ty;
        Model -> Proto: $model_to_proto:expr,
        Model <- Proto: $proto_to_model:expr$(,)?
    ) => {
        impl From<$Model> for $Proto {
            fn from(value: $Model) -> Self {
                type Model = $Model;
                type Proto = $Proto;

                $model_to_proto(value)
            }
        }

        impl TryFrom<$Proto> for $Model {
            type Error = ConversionError;

            fn try_from(value: $Proto) -> Result<Self, Self::Error> {
                type Model = $Model;
                type Proto = $Proto;

                type Error = ConversionErrorBuilder<Proto, Model>;

                $proto_to_model(value)
            }
        }
    }
}
