use std::process::Command;

use super::WrapperCommandExt;
use super::WrapperFixture;
use super::package_fixture::{
    create_artifact_api_response, create_fake_curl_bin, create_fake_curl_bin_with_release_package,
    create_runtime_package, create_source_layout_plugin, install_cached_runtime,
    install_legacy_cached_runtime,
};

#[path = "package/cache_precedence.rs"]
mod cache_precedence;
#[path = "package/default_discovery.rs"]
mod default_discovery;
#[path = "package/explicit_package.rs"]
mod explicit_package;
#[path = "package/override_isolation.rs"]
mod override_isolation;

pub(crate) use cache_precedence::*;
pub(crate) use default_discovery::*;
pub(crate) use explicit_package::*;
pub(crate) use override_isolation::*;
