use anyhow::Result;
use clap::{Parser, Subcommand};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::{
    fs::create_dir_all,
    io::{self, Error, ErrorKind},
};

static FORBIDDEN_CHARS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
static BEAN_CONFIG: &str = "config.toml";
static MKOSI_KERNEL: &str = "mkosi-kernel";
static MKOSI_KERNEL_PROFILES_DIR: &str = "mkosi.profiles";
static LINUX: &str = "linux";
static BTRFS_PROGS: &str = "btrfs-progs";
static FSTESTS: &str = "fstests";

/// a mkosi-kernel factory
#[derive(Parser)]
#[command(version, about)]
struct Beans {
    /// Absolute path to mkosi-kernel
    #[arg(long, env)]
    mkosi_kernel_dir: std::path::PathBuf,

    /// Absolute path to linux
    #[arg(long, env)]
    linux_dir: Option<std::path::PathBuf>,

    /// Absolute path to btrfs-progs
    #[arg(long, env)]
    btrfs_progs_dir: Option<std::path::PathBuf>,

    /// Absolute path to fstests
    #[arg(long, env)]
    fstests_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a new bean
    Configure {
        /// Name of your new bean
        #[arg(env)]
        bean_name: String,

        /// Run interactively
        #[arg(short, long)]
        interactive: bool,

        /// mkosi-kernel branch
        #[arg(short, long)]
        mkosi_kernel_branch: Option<String>,

        /// mkosi-kernel profile
        #[arg(short = 'p', long)]
        mkosi_kernel_profile: Option<String>,

        /// Linux branch
        #[arg(short, long)]
        linux_branch: Option<String>,

        /// btrfs-progs branch
        #[arg(short, long)]
        btrfs_progs_branch: Option<String>,

        /// fstest branch
        #[arg(short, long)]
        fstests_branch: Option<String>,
    },
    /// Sync modules in bean
    Sync {
        /// Name of your new bean
        #[arg(env)]
        bean_name: String,

        /// sync all modules
        #[arg(short, long)]
        all: bool,

        /// mkosi-kernel branch
        #[arg(short, long)]
        mkosi_kernel: bool,

        /// Linux branch
        #[arg(short, long, env)]
        linux: bool,

        /// btrfs-progs branch
        #[arg(short, long, env)]
        btrfs_progs: bool,

        /// fstest branch
        #[arg(short, long, env)]
        fstests: bool,
    },
    /// List beans
    #[clap(alias = "ls")]
    List,
    /// Mkosi wrapper for bean
    Mkosi {
        /// Name of your new bean
        #[arg(env)]
        bean_name: String,

        /// Do not include the profile in mkosi args, included by default
        #[arg(short, long)]
        no_profile: bool,

        /// Arguments to pass down to mkosi
        #[arg(trailing_var_arg = true)]
        mkosi_args: Vec<String>,
    },
    /// Fast fstests wrapper for bean
    FastFstests {
        /// Name of your new bean
        #[arg(env)]
        bean_name: String,

        /// fast-fstests dir
        #[arg(long, env)]
        fast_fstests_dir: std::path::PathBuf,

        /// Arguments to pass down to fast-fstests
        #[arg(trailing_var_arg = true)]
        fast_fstests_arg: Vec<String>,
    },
}

#[derive(Serialize, Deserialize)]
struct BeanConfig {
    name: String,
    mkosi_kernel_profile: Option<String>,
    mkosi_kernel_path: Option<PathBuf>,
    linux_path: Option<PathBuf>,
    btrfs_progs_path: Option<PathBuf>,
    fstest_path: Option<PathBuf>,
}

impl BeanConfig {
    fn new(name: &str) -> Self {
        BeanConfig {
            name: name.to_owned(),
            mkosi_kernel_profile: None,
            mkosi_kernel_path: None,
            linux_path: None,
            btrfs_progs_path: None,
            fstest_path: None,
        }
    }
}

fn create_new_bean(
    beans_config_dir: &std::path::PathBuf,
    name: &str,
) -> Result<std::path::PathBuf> {
    println!(
        "--- setting up new bean {:?} at {:?} ---",
        name, beans_config_dir
    );
    if name.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "profile name contains invalid characters",
        )
        .into())
    } else {
        let mut bean_dir = beans_config_dir.clone();
        bean_dir.push(name);
        create_dir_all(&bean_dir)?;
        Ok(bean_dir)
    }
}

fn list_beans(beans_config_dir: &std::path::PathBuf) -> Result<()> {
    println!("--- listing beans ---");

    let read_dir = beans_config_dir.read_dir()?;
    read_dir.for_each(|dir| {
        if let Ok(dir_entry) = dir {
            if let Ok(file_name) = dir_entry.file_name().into_string() {
                if !file_name.starts_with(".") && !file_name.ends_with(".env") {
                    println!("{}", file_name);
                }
            }
        }
    });

    Ok(())
}

fn git_clone_module_to_bean(
    bean_dir: &std::path::PathBuf,
    module_dir: &std::path::PathBuf,
    module_name: &str,
    module_branch: &str,
) -> Result<()> {
    println!(
        "--- cloning {:?} on branch {} ---",
        module_dir, module_branch
    );
    Command::new("git")
        .current_dir(bean_dir)
        .arg("clone")
        .arg(module_dir)
        .arg(module_name)
        .status()?;

    let module_dir = bean_dir.join(module_name);

    Command::new("git")
        .current_dir(&module_dir)
        .arg("switch")
        .arg("-C")
        .arg("tracker")
        .arg(&format!("origin/{}", module_branch))
        .status()?;

    Command::new("git")
        .current_dir(&module_dir)
        .arg("pull")
        .arg("--rebase")
        .status()?;

    Ok(())
}

fn list_module_git_branches(module_dir: &std::path::PathBuf) -> Result<()> {
    Command::new("git")
        .current_dir(&module_dir)
        .arg("branch")
        .arg("-v")
        .status()?;
    Ok(())
}

fn interactively_configure_module_branch(
    module_dir: &std::path::PathBuf,
    module_name: &str,
) -> Result<Option<String>> {
    println!("\n--- setting up {:?} module ---", module_name);
    println!("\nwould you like to configure this module?  [Y/n]");
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;

    match buf.trim().to_lowercase().as_str() {
        "y" | "" => {
            list_module_git_branches(module_dir)?;
            println!("\nwhich branch would you like to use?");
            buf = String::new();
            io::stdin().read_line(&mut buf)?;
            Ok(Some(String::from(buf.trim())))
        }
        "n" => Ok(None),
        _ => interactively_configure_module_branch(module_dir, module_name),
    }
}

fn list_mkosi_profiles(mkosi_kernel_dir: &std::path::PathBuf) -> Result<()> {
    let mkosi_kernel_profiles_dir = mkosi_kernel_dir.join(MKOSI_KERNEL_PROFILES_DIR);
    mkosi_kernel_profiles_dir.read_dir()?.for_each(|path| {
        if let Ok(path) = path {
            if let Ok(file_name) = path.file_name().into_string() {
                if let Some(profile) = file_name.strip_suffix(".conf") {
                    println!("{}", profile);
                }
            }
        }
    });

    Ok(())
}

fn interactively_get_mkosi_profile(
    mkosi_kernel_dir: &std::path::PathBuf,
) -> Result<Option<String>> {
    println!("\nwould you like to set a specific mkosi profile?  [Y/n]");
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;

    match buf.trim().to_lowercase().as_str() {
        "y" | "" => {
            list_mkosi_profiles(mkosi_kernel_dir)?;
            println!("\nwhich profile?");
            buf = String::new();
            io::stdin().read_line(&mut buf)?;
            Ok(Some(String::from(buf.trim())))
        }
        "n" => Ok(None),
        _ => interactively_get_mkosi_profile(mkosi_kernel_dir),
    }
}

fn configure_bean_module(
    interactive: &bool,
    bean_dir: &std::path::PathBuf,
    local_dir: Option<&std::path::PathBuf>,
    module_name: &str,
    local_branch: Option<&String>,
) -> Result<Option<std::path::PathBuf>> {
    match (interactive, local_dir, local_branch) {
        (true, Some(local_dir), None) => {
            if let Some(local_branch) =
                interactively_configure_module_branch(local_dir, module_name)?
            {
                git_clone_module_to_bean(&bean_dir, local_dir, module_name, &local_branch)?;
                Ok(Some(bean_dir.join(module_name)))
            } else {
                Ok(None)
            }
        }
        (_, Some(local_dir), Some(local_branch)) => {
            git_clone_module_to_bean(&bean_dir, local_dir, module_name, local_branch)?;
            Ok(Some(bean_dir.join(module_name)))
        }
        (_, _, _) => Ok(None),
    }
}

fn save_bean_config(bean_dir: &std::path::PathBuf, bean_config: &BeanConfig) -> Result<()> {
    let data = toml::to_string_pretty(bean_config)?;
    let mut bean_config_file = File::create(bean_dir.join(BEAN_CONFIG))?;
    bean_config_file.write(data.as_bytes())?;
    Ok(())
}

fn read_bean_config(bean_dir: &std::path::PathBuf) -> Result<BeanConfig> {
    Ok(toml::from_str(&std::fs::read_to_string(
        bean_dir.join(BEAN_CONFIG),
    )?)?)
}

fn sync_bean_helper(sync: bool, bean_dir: &std::path::PathBuf, module_name: &str) -> Result<bool> {
    let module_dir = bean_dir.join(module_name);
    if !module_dir.exists() {
        return Ok(false);
    }
    match (sync, module_dir) {
        (true, module_dir) => {
            println!("--- syncing module {} ---", module_name);
            Command::new("git")
                .current_dir(&module_dir)
                .arg("pull")
                .arg("--rebase")
                .status()?;
            Ok(true)
        }
        (_, _) => Ok(false),
    }
}

fn main() {
    let beans_config_dir = config_dir().unwrap().join("beans");
    dotenv::from_path(beans_config_dir.join("beans.env")).ok();
    let beans = Beans::parse();

    match beans.command {
        Commands::Configure {
            bean_name,
            interactive,
            mkosi_kernel_branch,
            mkosi_kernel_profile,
            linux_branch,
            btrfs_progs_branch,
            fstests_branch,
        } => {
            let bean_dir = create_new_bean(&beans_config_dir, &bean_name).unwrap();
            let mut bean_config = BeanConfig::new(&bean_name);

            bean_config.mkosi_kernel_path = configure_bean_module(
                &interactive,
                &bean_dir,
                Some(&beans.mkosi_kernel_dir),
                MKOSI_KERNEL,
                mkosi_kernel_branch.as_ref(),
            )
            .unwrap();

            bean_config.mkosi_kernel_profile = if interactive {
                interactively_get_mkosi_profile(&beans.mkosi_kernel_dir).unwrap()
            } else {
                mkosi_kernel_profile
            };

            bean_config.linux_path = configure_bean_module(
                &interactive,
                &bean_dir,
                beans.linux_dir.as_ref(),
                LINUX,
                linux_branch.as_ref(),
            )
            .unwrap();

            bean_config.btrfs_progs_path = configure_bean_module(
                &interactive,
                &bean_dir,
                beans.btrfs_progs_dir.as_ref(),
                BTRFS_PROGS,
                btrfs_progs_branch.as_ref(),
            )
            .unwrap();

            bean_config.fstest_path = configure_bean_module(
                &interactive,
                &bean_dir,
                beans.fstests_dir.as_ref(),
                FSTESTS,
                fstests_branch.as_ref(),
            )
            .unwrap();

            save_bean_config(&bean_dir, &bean_config).unwrap();
        }
        Commands::Sync {
            bean_name,
            all,
            mut mkosi_kernel,
            mut linux,
            mut btrfs_progs,
            mut fstests,
        } => {
            let bean_dir = beans_config_dir.join(&bean_name);
            if all {
                mkosi_kernel = true;
                linux = true;
                btrfs_progs = true;
                fstests = true;
            }
            sync_bean_helper(mkosi_kernel, &bean_dir, MKOSI_KERNEL).unwrap();
            sync_bean_helper(linux, &bean_dir, LINUX).unwrap();
            sync_bean_helper(btrfs_progs, &bean_dir, BTRFS_PROGS).unwrap();
            sync_bean_helper(fstests, &bean_dir, FSTESTS).unwrap();
        }
        Commands::List => {
            list_beans(&beans_config_dir).unwrap();
        }
        Commands::Mkosi {
            bean_name,
            no_profile,
            mut mkosi_args,
        } => {
            let bean_dir = beans_config_dir.join(&bean_name);
            let bean_config = read_bean_config(&bean_dir).unwrap();

            let mut build_sources = vec![];
            if let Some(linux_path) = bean_config.linux_path {
                build_sources.push(format!("{}:{}", linux_path.to_string_lossy(), LINUX));
            }
            if let Some(btrfs_progs_path) = bean_config.btrfs_progs_path {
                build_sources.push(format!(
                    "{}:{}",
                    btrfs_progs_path.to_string_lossy(),
                    BTRFS_PROGS
                ));
            }
            if let Some(fstests_path) = bean_config.fstest_path {
                build_sources.push(format!("{}:{}", fstests_path.to_string_lossy(), FSTESTS));
            }
            mkosi_args.insert(0, format!("--build-sources={}", build_sources.join(",")));

            if !no_profile {
                if let Some(mkosi_kernel_profile) = bean_config.mkosi_kernel_profile {
                    println!(
                        "--- using mkosi-kernel-profile {} ---",
                        mkosi_kernel_profile
                    );
                    mkosi_args.insert(0, format!("--profile={}", mkosi_kernel_profile));
                }
            }

            println!("{:?}", mkosi_args);

            if let Some(mkosi_kernel_dir) = bean_config.mkosi_kernel_path {
                Command::new("mkosi")
                    .current_dir(mkosi_kernel_dir)
                    .args(mkosi_args)
                    .status()
                    .unwrap();
            } else {
                println!("mkosi-kernel not configured for bean {}", bean_config.name);
            }
        }
        Commands::FastFstests {
            bean_name,
            fast_fstests_dir,
            fast_fstests_arg,
        } => {
            let mkosi_kernel_dir = beans_config_dir.join(bean_name).join(MKOSI_KERNEL);
            Command::new("pytest")
                .current_dir(fast_fstests_dir)
                .arg("src/fast-fstests.py")
                .arg("--mkosi-config-dir")
                .arg(mkosi_kernel_dir)
                .args(fast_fstests_arg)
                .status()
                .unwrap();
        }
    }
}
