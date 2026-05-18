use anyhow::Result;
use std::env;

pub struct Config {
    pub ape_key: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let ape_key = env::var("MONKEYTYPE_APE_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        Ok(Self { ape_key })
    }
}
