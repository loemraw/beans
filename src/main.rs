use std::{env::current_dir, io::Read};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use crate::kernel::Kernel;
use crate::mkosi_kernel::MkosiKernel;

mod kernel;
mod mkosi_kernel;
mod module;
mod util;

const BEAN_CONFIG_FILE: &str = ".bean.config.toml";

#[derive(Serialize, Deserialize, Debug)]
struct BeanConfig {
    kernel: Kernel,
    mkosi_kernel: MkosiKernel,
}

#[derive(Parser)]
struct CLI {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        bean: std::path::PathBuf,

        #[clap(short = 'c', long)]
        from_config: Option<std::path::PathBuf>,
    },
    Sync {
        #[clap(default_value=get_current_bean())]
        bean: std::path::PathBuf,

        modules: Vec<String>,

        #[clap(short, long)]
        all: bool,
    },
    Mkosi {
        #[clap(default_value=get_current_bean())]
        bean: std::path::PathBuf,

        #[clap(last=true)]
        mkosi_args: Vec<String>,
    },
}

fn get_current_bean() -> Option<&'static str> {
    let current_dir = current_dir().ok()?;
    let mut path = Some(current_dir.as_path());
    while let Some(curr) = path {
        let bean_config_path = curr.join(BEAN_CONFIG_FILE);
        if bean_config_path.exists() {
            return Some(Box::leak(Box::new(bean_config_path.to_str()?.to_string())));
        }
        path = curr.parent();
    }
    None
}

fn main() {
    CLI::parse();
    println!("Hello world");
}
