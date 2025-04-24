use std::error::Error;

use serde::{Deserialize, Serialize};

pub(crate) trait Module<'de>: Serialize + Deserialize<'de> {
    fn load(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>>;
    fn sync(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>>;
    fn unload(&mut self, bean_path: &std::path::Path) -> Result<(), Box<dyn Error>>;
}
