use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::{
    module::Module,
    util::{Expectations, bean_name_from_, git_branch, git_hash, git_status},
};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MkosiKernel {
    source_path: std::path::PathBuf,
    bean_relative_path: std::path::PathBuf,
    profile: String,
}

impl MkosiKernel {
    fn setup(
        source_path: &std::path::Path,
        bean_relative_path: &std::path::Path,
        profile: &str,
    ) -> Self {
        Mkosi {
            source_path: source_path.to_path_buf(),
            bean_relative_path: bean_relative_path.to_path_buf(),
            profile: profile.to_string(),
        }
    }
}

impl Module<'_> for Mkosi {
    fn load(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn sync(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn unload(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
