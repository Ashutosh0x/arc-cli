// SPDX-License-Identifier: MIT
pub mod auth;

pub use auth::{
    require_auth, AuthConfig, AuthenticatedIdentity, ReplayCache,
};
