#![allow(clippy::all)]

pub(crate) mod generated {
    pub(crate) mod push_secret {
        include!(concat!(env!("OUT_DIR"), "/push_secret.rs"));
    }

    pub(crate) mod external_secret {
        include!(concat!(env!("OUT_DIR"), "/external_secret.rs"));
    }

    pub(crate) mod ech_board_network {
        include!(concat!(env!("OUT_DIR"), "/ech_board_network.rs"));
    }
}

pub(crate) use generated::ech_board_network::*;
pub(crate) use generated::external_secret::*;
pub(crate) use generated::push_secret::*;
