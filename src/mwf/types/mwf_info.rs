/// Information about the Mesh Worker Framework.
#[derive(Debug, Clone, Copy)]
pub struct MWFInfo {
    /// crate name.
    pub crate_name: &'static str,

    /// crate version.
    pub crate_version: &'static str,

    /// Git commit used to build the crate.
    pub crate_git_commit: &'static str,

    /// Build profile ("debug" or "release").
    pub build_profile: &'static str,
}

impl std::fmt::Display for MWFInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        writeln!(f, "===============================")?;
        writeln!(f, "     {}", self.crate_name.to_uppercase())?;
        writeln!(f, "===============================")?;
        writeln!(f, "Version : {}", self.crate_version)?;
        writeln!(f, "Commit  : {}", self.crate_git_commit)?;
        writeln!(f, "Profile : {}", self.build_profile)?;
        writeln!(f, "===============================")
    }
}
