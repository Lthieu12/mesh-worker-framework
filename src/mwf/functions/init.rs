use super::super::MWF;
use super::super::MWFConfiguration;
use crate::mwf::MWFError;

impl MWF {
    /// Initialize MWF using the default configuration.
    ///
    /// Each successful initialization after a previous shutdown receives a new
    /// instance ID, used to ensure WorkerID uniqueness within the current process.
    ///
    /// # Examples
    ///
    #[doc = concat!(
        "### getting_started\n\n",
        "```rust,ignore\n",
        include_str!("../../../examples/getting_started/main.rs"),
        "\n```\n",
        "### demo_worker\n\n",
        "```rust,ignore\n",
        include_str!("../../../examples/getting_started/workers/demo_worker.rs"),
        "\n```\n\n",
    )]
    pub fn init() -> Result<(), MWFError> {
        Self::init_with_configuration(MWFConfiguration::default())
    }
}
