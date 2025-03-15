use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::GeneratorError;

pub fn write_filename(name: &PathBuf, content: &str) -> Result<(), GeneratorError> {
    fs::create_dir_all(&name.parent().unwrap()).expect("Creating objects dir failed");
    let mut object_file = match File::create(name) {
        Ok(file) => file,
        Err(err) => {
            return Err(GeneratorError::FileCreationError(
                name.as_os_str().to_string_lossy().to_string(),
                err.to_string(),
            ))
        }
    };
    object_file.write(content.as_bytes()).unwrap();
    Ok(())
}
