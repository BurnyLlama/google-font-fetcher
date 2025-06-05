use inline_colorization::*;
use std::{
    fs::{File, create_dir_all},
    io::Write,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::exit_codes::{
    EXIT_CODE_FILE_IO_ERROR, EXIT_CODE_INVALID_FONT_MANIFEST, EXIT_CODE_NET_ERROR,
};

/// Find the base path for the font files.
pub fn get_font_base_path() -> String {
    let home_dir = std::env::var("HOME").unwrap_or("~".to_string());
    let xdg_data_home =
        std::env::var("XDG_DATA_HOME").unwrap_or(format!("{}/.local/share", home_dir));
    match std::env::var("FONTY_BASE_PATH") {
        Ok(env_var) => env_var,
        Err(_) => format!("{}/fonts/Google", xdg_data_home),
    }
}

/// A file with its contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestFile {
    filename: String,
    contents: String,
}

/// A reference to a file.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestFileRef {
    filename: String,
    url: String,
}

/// A manifest of files and file references. Files contain their data in the manifest. File references contain a url to the file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontManifest {
    files: Vec<ManifestFile>,
    #[serde(rename = "fileRefs")]
    file_refs: Vec<ManifestFileRef>,
}

/// The outer most layer that Google Fonts return.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct FontManifestWrapper {
    #[serde(rename = "zipName")]
    zip_name: String,
    manifest: FontManifest,
}

impl FontManifest {
    /// Gets the font manifest from Google Fonts.
    pub fn fetch(font_names: Vec<&str>) -> Result<FontManifest, reqwest::Error> {
        // The format for getting fonts from Google Fonts is ?family=font1,font2,font3,...
        let response = reqwest::blocking::get(format!(
            "https://fonts.google.com/download/list?family={}",
            font_names.join(",")
        ))?;

        let text = response.text()?;
        // For some reason Google Fonts adds some nasty extra characters before the JSON payload, remove them.
        let json = text.replace(")]}'\n", "");

        // Parse the JSON into a valid FontManifest struct.
        let font_manifest_wrapper: FontManifestWrapper = match serde_json::from_str(&json) {
            Ok(font_manifest) => font_manifest,
            Err(e) => {
                println!("Invalid FontManifest! Error:\n{}", e);
                std::process::exit(EXIT_CODE_INVALID_FONT_MANIFEST);
            }
        };

        Ok(font_manifest_wrapper.manifest)
    }

    /// Prepends a path to the file paths in the manifest.
    pub fn prepand_path_to_files(self, path: &str) -> Self {
        Self {
            files: self
                .files
                .iter()
                .map(|file| ManifestFile {
                    filename: format!("{}/{}", path, file.filename),
                    contents: file.contents.clone(),
                })
                .collect(),
            file_refs: self
                .file_refs
                .iter()
                .map(|file| ManifestFileRef {
                    filename: format!("{}/{}", path, file.filename),
                    url: file.url.clone(),
                })
                .collect(),
        }
    }

    /// Checks if a font is a valid font on Google Fonts.
    pub fn check_if_valid_font(font_name: &str) -> bool {
        // A font is valid if /specimen/font-name can be reached and is a success.
        reqwest::blocking::get(format!(
            "https://fonts.google.com/specimen/{}",
            font_name.replace(" ", "+")
        ))
        .is_ok_and(|response| response.status().is_success())
    }

    /// Write files (with their contents in the manifest) to disk.
    pub fn write_files(&self) {
        for file in &self.files {
            let raw_filepath = format!("{}/{}", get_font_base_path(), file.filename);
            let filepath = Path::new(&raw_filepath);

            let parent_dir = match filepath.parent() {
                Some(parent_dir) => parent_dir,
                None => {
                    println!("Invalid file path: '{}'", raw_filepath);
                    std::process::exit(EXIT_CODE_INVALID_FONT_MANIFEST);
                }
            };

            // Equivalent to `mkdir -p`.
            match create_dir_all(parent_dir) {
                Ok(_) => (),
                Err(e) => {
                    println!(
                        "Failed to create directory: '{:?}'! Error:\n{}",
                        parent_dir, e
                    );
                    std::process::exit(EXIT_CODE_INVALID_FONT_MANIFEST);
                }
            }

            // Create the file, and get reference to it.
            let mut file_writer = match File::create(filepath) {
                Ok(file_writer) => file_writer,
                Err(e) => {
                    println!("Failed to create file: '{:?}'! Error:\n{}", filepath, e);
                    std::process::exit(EXIT_CODE_FILE_IO_ERROR);
                }
            };

            // Write the contents to the file.
            match file_writer.write_all(file.contents.as_bytes()) {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to write to file: '{:?}'! Error:\n{}", filepath, e);
                    std::process::exit(EXIT_CODE_FILE_IO_ERROR);
                }
            };
        }
    }

    /// Fetches the files from the file references.
    pub fn fetch_files_from_refs(&self) {
        let downloads = self.file_refs.len();
        for (index, file_ref) in self.file_refs.iter().enumerate() {
            print!(
                "{color_cyan}INFO:{color_reset} Downloading file {color_bright_yellow}{}{color_white}/{}{color_reset}: {color_blue}'{}' {color_bright_black}... ",
                index + 1,
                downloads,
                file_ref.filename
            );

            let response = match reqwest::blocking::get(&file_ref.url) {
                Ok(response) => response,
                Err(e) => {
                    println!("Failed to fetch file: '{:?}'! Error:\n{}", file_ref.url, e);
                    std::process::exit(EXIT_CODE_NET_ERROR);
                }
            };

            // If the request failed, exit.
            if !response.status().is_success() {
                println!(
                    "Failed to fetch file: '{:?}'! Got status '{}'.",
                    file_ref.url,
                    response.status()
                );
                std::process::exit(EXIT_CODE_NET_ERROR);
            }

            let file_bytes = match response.bytes() {
                Ok(file_bytes) => file_bytes,
                Err(e) => {
                    println!("Failed to fetch file: '{:?}'! Error:\n{}", file_ref.url, e);
                    std::process::exit(EXIT_CODE_NET_ERROR);
                }
            };

            let raw_filepath = format!("{}/{}", get_font_base_path(), file_ref.filename);
            let filepath = Path::new(&raw_filepath);

            let parent_dir = match filepath.parent() {
                Some(parent_dir) => parent_dir,
                None => {
                    println!("Invalid file path: '{}'", raw_filepath);
                    std::process::exit(EXIT_CODE_INVALID_FONT_MANIFEST);
                }
            };

            // Equivalent to `mkdir -p`.
            match create_dir_all(parent_dir) {
                Ok(_) => (),
                Err(e) => {
                    println!(
                        "Failed to create directory: '{:?}'! Error:\n{}",
                        parent_dir, e
                    );
                    std::process::exit(EXIT_CODE_INVALID_FONT_MANIFEST);
                }
            }

            // Create the file, and get reference to it.
            let mut file_writer = match File::create(filepath) {
                Ok(file_writer) => file_writer,
                Err(e) => {
                    println!("Failed to create file: '{:?}'! Error:\n{}", filepath, e);
                    std::process::exit(EXIT_CODE_FILE_IO_ERROR);
                }
            };

            // Write the contents to the file.
            match file_writer.write_all(&file_bytes) {
                Ok(_) => (),
                Err(e) => {
                    println!("Failed to write to file: '{:?}'! Error:\n{}", filepath, e);
                    std::process::exit(EXIT_CODE_FILE_IO_ERROR);
                }
            };
            println!("{color_green}DONE!{color_reset}");
        }
    }
}
