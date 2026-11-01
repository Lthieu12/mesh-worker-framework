use super::super::MWF;
use super::super::MWFInfo;

impl MWF {
    /// Returns information about the Mesh Worker Framework.
    pub fn get_info() -> MWFInfo {
        MWFInfo { crate_name: env!("CARGO_PKG_NAME"), crate_version: env!("CARGO_PKG_VERSION"), crate_git_commit: env!("MWF_GIT_COMMIT"), build_profile: if cfg!(debug_assertions) { "debug" } else { "release" } }
    }
}
