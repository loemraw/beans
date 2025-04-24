use std::error::Error;

pub(crate) trait Expectations {
    fn expect_success(&self) -> Result<(), Box<dyn Error>>;
    fn expect(&self, code: &[i32]) -> Result<(), Box<dyn Error>>;
}

impl Expectations for std::process::ExitStatus {
    fn expect_success(&self) -> Result<(), Box<dyn Error>> {
        if self.success() {
            Ok(())
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "expected success",
            )))
        }
    }

    fn expect(&self, code: &[i32]) -> Result<(), Box<dyn Error>> {
        let actual = self.code().ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unable to get exit code for process",
        ))?;

        for &c in code {
            if actual == c {
                return Ok(());
            }
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "unexpected error code",
        )))
    }
}

pub(crate) fn bean_name_from_(
    bean_path: &std::path::Path,
) -> Result<&std::ffi::OsStr, Box<dyn Error>> {
    bean_path.file_name().ok_or(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("unable to get bean name from bean_path, {:?}", bean_path),
    )))
}

pub(crate) fn git_branch(path: &std::path::Path) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .stdout(std::process::Stdio::piped())
        .arg("branch")
        .arg("--show-current")
        .output()?;

    output.status.expect_success()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) fn git_hash(path: &std::path::Path) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .stdout(std::process::Stdio::piped())
        .arg("log")
        .arg("--pretty=%H")
        .output()?;

    output.status.expect_success()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) fn git_status(path: &std::path::Path) -> Result<bool, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .stdout(std::process::Stdio::piped())
        .arg("log")
        .arg("--pretty=%H")
        .output()?;

    output.status.expect_success()?;

    Ok(output.stdout.len() > 0)
}
