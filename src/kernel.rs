use std::error::Error;

use serde::{Deserialize, Serialize};

use crate::{
    module::Module,
    util::{Expectations, bean_name_from_, git_branch, git_hash, git_status},
};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Kernel {
    source_path: std::path::PathBuf,
    clean_path: std::path::PathBuf,
    bean_relative_dev_path: std::path::PathBuf,
    module_status: KernelStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum KernelStatus {
    Unloaded,
    Loaded { branch: String, hash: String },
}

impl Kernel {
    fn setup(
        source_path: &std::path::Path,
        clean_path: &std::path::Path,
        bean_relative_dev_path: &std::path::Path,
    ) -> Self {
        Kernel {
            source_path: source_path.to_path_buf(),
            clean_path: clean_path.to_path_buf(),
            bean_relative_dev_path: bean_relative_dev_path.to_path_buf(),
            module_status: KernelStatus::Unloaded,
        }
    }
}

impl Module<'_> for Kernel {
    fn load(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        match self.module_status {
            KernelStatus::Loaded { branch: _, hash: _ } => return Ok(()),
            KernelStatus::Unloaded => (),
        }

        let module_path = bean_path.join(&self.bean_relative_dev_path);

        std::process::Command::new("git")
            .current_dir(&self.source_path)
            .arg("worktree")
            .arg("add")
            .arg(&module_path)
            .arg("-b")
            .arg(bean_name_from_(bean_path)?)
            .status()?
            .expect_success()?;

        std::process::Command::new("git")
            .current_dir(&self.source_path)
            .arg("worktree")
            .arg("add")
            .arg(&self.clean_path)
            .arg("-d")
            .status()?
            .expect(&[0, 128])?;

        self.module_status = KernelStatus::Loaded {
            branch: git_branch(&module_path)?,
            hash: git_hash(&module_path)?,
        };

        Ok(())
    }

    fn sync(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        match self.module_status {
            KernelStatus::Unloaded => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "cannot sync an unloaded module",
                )));
            }
            KernelStatus::Loaded { branch: _, hash: _ } => (),
        }

        let module_path = bean_path.join(&self.bean_relative_dev_path);

        if !git_status(&module_path)? {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "working directory not clean... make sure to commit all changes",
            )));
        }

        let branch = git_branch(&module_path)?;

        std::process::Command::new("git")
            .current_dir(&self.clean_path)
            .arg("switch")
            .arg(&branch)
            .arg("--detach")
            .status()?
            .expect_success()?;

        self.module_status = KernelStatus::Loaded {
            branch,
            hash: git_hash(&module_path)?,
        };

        Ok(())
    }

    fn unload(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>> {
        match self.module_status {
            KernelStatus::Unloaded => return Ok(()),
            KernelStatus::Loaded { branch: _, hash: _ } => (),
        }

        let module_path = bean_path.join(&self.bean_relative_dev_path);

        self.module_status = KernelStatus::Unloaded;

        std::process::Command::new("git")
            .current_dir(&self.source_path)
            .arg("worktree")
            .arg("remove")
            .arg(&module_path)
            .status()?
            .expect_success()?;

        Ok(())
    }
}
