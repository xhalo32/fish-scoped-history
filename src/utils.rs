use std::{env, path::PathBuf};

pub(crate) fn get_default_data_directory() -> PathBuf {
    let data_home = env::var("XDG_DATA_HOME")
        .or_else(|_| env::var("HOME").map(|home| home + "/.local/share/fish"))
        .expect("XDG_DATA_HOME or HOME to be set");
    PathBuf::from(data_home)
}
