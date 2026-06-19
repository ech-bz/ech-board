pub(crate) mod fullnode;
pub(crate) mod validator;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct KeyPairValue {
    pub(crate) value: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct Empty {}
