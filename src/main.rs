use std::{
    env::current_dir,
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::{Read, Write},
    os::unix::fs::symlink,
    process::Command,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dirs::config_local_dir;
use serde::{Deserialize, Serialize};
use toml::Table;

const BEANS_CONFIG_DIR: &str = "beans";
const BEANS_CONFIG_FILE: &str = "beans.toml";
const BEAN_MARKER_FILE: &str = "bean.toml";
const BEANS_DIR_KEY: &str = "beans_dir";
const BEANS_CURRENT_KEY: &str = "current_dir";
const BEANS_CONFIG_MODULES: &str = "modules";
const BEANS_CONFIG_MKOSI_KERNEL: &str = "mkosi_kernel_name";

fn default_config_path() -> Option<&'static str> {
    let config_path = config_local_dir()?
        .join(BEANS_CONFIG_DIR)
        .join(BEANS_CONFIG_FILE)
        .to_string_lossy()
        .to_string();
    Some(Box::leak(Box::new(config_path)))
}

fn default_bean() -> Option<&'static str> {
    let current_dir = current_dir().ok()?;
    let mut path = Some(current_dir.as_path());
    while let Some(curr) = path {
        let bean_marker_path = curr.join(BEAN_MARKER_FILE);
        if bean_marker_path.exists() {
            let mut bean_marker_file = std::fs::File::open(bean_marker_path).ok()?;
            let mut buf = String::new();
            bean_marker_file.read_to_string(&mut buf).ok()?;
            let bean_config: BeanConfig = toml::from_str(&buf).ok()?;
            return Some(Box::leak(Box::new(bean_config.name)));
        }
        path = curr.parent();
    }
    None
}

#[derive(Serialize, Deserialize, Debug)]
struct BeanConfig {
    name: String,
    modules: Vec<Module>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Module {
    name: String,
    path: std::path::PathBuf,
    branch: Option<String>,
}

#[derive(Parser, Debug)]
struct CLI {
    #[clap(short, long, default_value=default_config_path())]
    config_file: std::path::PathBuf,

    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    New {
        bean: String,

        #[clap(short = 'm', long)]
        only_include_modules: Option<Vec<String>>,

        #[clap(short = 'M', long)]
        exclude_modules: Option<Vec<String>>,
    },
    Delete {
        #[clap(default_value=default_bean())]
        bean: String,
    },
    Set {
        #[clap(default_value=default_bean())]
        bean: String,
    },
    Mkosi {
        #[clap(default_value=default_bean())]
        bean: String,

        #[clap(trailing_var_arg = true)]
        mkosi_args: Vec<String>,
    },
}

fn create_bean(beans_dir: &std::path::Path, bean_config: &BeanConfig) -> Result<()> {
    println!("--- creating bean {} ---", &bean_config.name);
    let bean_dir = beans_dir.join(&bean_config.name);
    create_dir_all(&bean_dir)?;
    let mut bean_marker_file = File::create_new(bean_dir.join(BEAN_MARKER_FILE))?;
    bean_marker_file.write_all(toml::to_string_pretty(bean_config)?.as_bytes())?;

    for module in bean_config.modules.iter() {
        add_module(&bean_dir, &bean_config.name, &module)?;
    }

    Ok(())
}

fn add_module(bean_dir: &std::path::Path, bean_name: &str, module: &Module) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(&module.path)
        .arg("worktree")
        .arg("add")
        .arg(bean_dir.join(&module.name));

    if let Some(branch) = &module.branch {
        cmd.arg(branch).arg("--track").arg("-B").arg(bean_name);
    } else {
        cmd.arg("-d");
    }

    println!("--- adding module {} ---", &module.name);
    cmd.status()?;

    Ok(())
}

fn get_bean_config(beans_dir: &std::path::Path, bean_name: &str) -> Result<BeanConfig> {
    let bean_dir = beans_dir.join(&bean_name);
    let mut bean_marker_file = File::open(bean_dir.join(BEAN_MARKER_FILE))?;
    let mut buf = String::new();
    bean_marker_file.read_to_string(&mut buf)?;

    Ok(toml::from_str(&buf)?)
}

fn delete_bean(beans_dir: &std::path::Path, bean_name: &str) -> Result<()> {
    println!("--- deleting bean {} ---", bean_name);

    let bean_dir = beans_dir.join(bean_name);
    let config = get_bean_config(beans_dir, bean_name)?;

    for module in config.modules.iter() {
        println!("--- deleting module {} ---", &module.name);
        Command::new("git")
            .current_dir(&module.path)
            .arg("worktree")
            .arg("remove")
            .arg(bean_dir.join(&module.name))
            .status()?
            .success()
            .then_some(())
            .context(format!("unable to delete module {}", module.name))?;
    }

    remove_dir_all(bean_dir)?;

    Ok(())
}

fn set_bean(
    beans_dir: &std::path::Path,
    bean_name: &str,
    current_directory: &std::path::Path,
) -> Result<()> {
    if let Some(parent) = current_directory.parent() {
        if parent.exists() {
            create_dir_all(&parent)?;
        }
    }

    remove_file(current_directory)?;
    symlink(beans_dir.join(bean_name), current_directory)?;

    Ok(())
}

fn main() -> Result<()> {
    let cli = CLI::parse();
    let mut config_file = std::fs::File::open(cli.config_file)?;
    let mut buf = String::new();
    config_file.read_to_string(&mut buf)?;
    let config = buf.parse::<Table>()?;

    let beans_dir = std::path::Path::new(
        config
            .get(BEANS_DIR_KEY)
            .context(format!("{} not specified in config file.", BEANS_DIR_KEY))?
            .as_str()
            .context("unable to convert config file to string")?,
    );

    let current_directory = std::path::Path::new(
        config
            .get(BEANS_CURRENT_KEY)
            .context(format!("{} not specified in config file.", BEANS_DIR_KEY))?
            .as_str()
            .context("unable to convert config file to string")?,
    );

    let mkosi_name = config
        .get(BEANS_CONFIG_MKOSI_KERNEL)
        .context(format!("{} not specified in config file.", BEANS_DIR_KEY))?
        .as_str()
        .context("unable to convert config file to string")?;

    match cli.commands {
        Commands::New {
            bean,
            only_include_modules,
            exclude_modules,
        } => {
            let modules: Vec<Module> = config
                .get(BEANS_CONFIG_MODULES)
                .context(format!(
                    "{} not specified in config file",
                    BEANS_CONFIG_MODULES
                ))?
                .to_owned()
                .try_into::<Vec<Module>>()?
                .iter()
                .filter_map(|module| {
                    if let Some(only_include_modules) = &only_include_modules {
                        if !only_include_modules.contains(&module.name) {
                            return None;
                        }
                    }
                    if let Some(exclude_modules) = &exclude_modules {
                        if exclude_modules.contains(&module.name) {
                            return None;
                        }
                    }
                    Some(module.to_owned())
                })
                .collect();

            let bean_config = BeanConfig {
                name: bean,
                modules,
            };
            create_bean(beans_dir, &bean_config)?;
        }
        Commands::Delete { bean } => {
            delete_bean(beans_dir, &bean)?;
        }
        Commands::Set { bean } => {
            set_bean(beans_dir, &bean, current_directory)?;
        }
        Commands::Mkosi { bean, mkosi_args } => {
            set_bean(beans_dir, &bean, current_directory)?;

            Command::new("mkosi")
                .current_dir(current_directory.join(mkosi_name))
                .args(mkosi_args)
                .status()?;
        }
    }

    Ok(())
}
