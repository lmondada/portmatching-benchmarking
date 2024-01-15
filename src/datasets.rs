use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::utils::{json_to_qasm, qasm_to_json};
use hugr::Hugr;
use tket2::{json::load_tk1_json_str, portmatching::CircuitPattern};

pub mod ecc;

pub trait Dataset {
    type Graph;

    /// Generates the dataset.
    ///
    /// Returns the number of circuits generated.
    fn generate(&self, save_files: bool) -> usize;

    /// The circuits as qasm strings.
    fn iter_qasm(&self) -> impl Iterator<Item = String>;

    /// The circuits as json strings.
    fn iter_json(&self) -> impl Iterator<Item = String>;

    /// Dataset name.
    fn name(&self) -> String;
}

pub trait CircuitDataset {
    fn from_circuit_folder(folder: &Path) -> Self;
    fn circuit_folder(&self) -> &Path;
}

struct UnsavedFile {
    path: PathBuf,
    contents: Option<String>,
    is_saved: bool,
}

impl UnsavedFile {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            contents: None,
            is_saved: false,
        }
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn is_empty(&self) -> bool {
        self.contents.is_none() && !self.exists()
    }

    fn save(&mut self) -> io::Result<()> {
        if self.is_saved {
            return Ok(());
        }
        let Some(contents) = &self.contents else {
            return Ok(());
        };
        self.is_saved = true;
        fs::write(&self.path, contents)
    }

    fn load(&mut self) -> io::Result<&str> {
        if !self.is_empty() {
            self.contents = Some(fs::read_to_string(&self.path)?);
            self.is_saved = true;
        }
        self.contents
            .as_ref()
            .map(|s| s.as_str())
            .ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "cannot load, file not found",
            ))
    }

    fn set_contents(&mut self, contents: String) {
        self.contents = Some(contents);
        self.is_saved = false;
    }
}

pub(crate) struct QasmAndJson {
    qasm: UnsavedFile,
    json: UnsavedFile,
}

impl QasmAndJson {
    pub(crate) fn from_path(path: &Path) -> Self {
        let qasm_path = path.with_extension("qasm");
        let json_path = path.with_extension("json");
        Self {
            qasm: UnsavedFile::new(qasm_path),
            json: UnsavedFile::new(json_path),
        }
    }

    /// Load both files, either from disk or by converting the other file.
    ///
    /// One of the files must exist.
    pub(crate) fn load(&mut self) -> io::Result<()> {
        if self.qasm.is_empty() {
            println!("{:?} -> {:?}", self.json.path, self.qasm.path);
            self.qasm.set_contents(json_to_qasm(self.json.load()?)?);
        } else if self.json.is_empty() {
            println!("{:?} -> {:?}", self.qasm.path, self.json.path);
            self.json.set_contents(qasm_to_json(self.qasm.load()?)?);
        } else {
            self.json.load()?;
            self.qasm.load()?;
        }
        Ok(())
    }

    pub(crate) fn qasm(&self) -> Option<&str> {
        self.qasm.contents.as_ref().map(|s| s.as_str())
    }

    pub(crate) fn json(&self) -> Option<&str> {
        self.json.contents.as_ref().map(|s| s.as_str())
    }

    fn save(&mut self) -> io::Result<()> {
        self.qasm.save()?;
        self.json.save()
    }

    fn needs_conversion(&self) -> bool {
        self.qasm.is_empty() || self.json.is_empty()
    }

    fn valid_pattern(&self) -> bool {
        let Some(json) = self.json.contents.as_ref() else {
            return false;
        };
        let circ = load_tk1_json_str(json).unwrap();
        CircuitPattern::try_from_circuit(&circ).is_ok()
    }
}

impl<T: CircuitDataset> Dataset for T {
    type Graph = Hugr;

    fn iter_qasm(&self) -> impl Iterator<Item = String> {
        let qasm_bin_file = fs::File::open(self.circuit_folder().join("qasm.bin")).unwrap();
        let qasm: Vec<String> = rmp_serde::decode::from_read(qasm_bin_file).unwrap();
        qasm.into_iter()
    }

    fn iter_json(&self) -> impl Iterator<Item = String> {
        let json_bin_file = fs::File::open(self.circuit_folder().join("json.bin")).unwrap();
        let json: Vec<String> = rmp_serde::decode::from_read(json_bin_file).unwrap();
        json.into_iter()
    }

    fn name(&self) -> String {
        self.circuit_folder()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    fn generate(&self, save_files: bool) -> usize {
        let folder = self.circuit_folder();

        let file_names: HashSet<PathBuf> = folder
            .read_dir()
            .expect("Failed to read directory")
            .map(|entry| entry.expect("Failed to read directory entry").path())
            .filter(|path| {
                path.extension()
                    .map_or(false, |ext| ext == "json" || ext == "qasm")
            })
            .map(|path| path.with_extension(""))
            .collect();

        let mut files: Vec<_> = file_names
            .into_iter()
            .map(|path| QasmAndJson::from_path(&path))
            .collect();

        let n_conversions = files.iter().filter(|path| path.needs_conversion()).count();
        if n_conversions > 100 {
            println!(
                "Warning: must convert {} files from and to QASM in {}. \
                This might take a while. Using the qasm_to_json.py script \
                manually is recommended.",
                n_conversions,
                folder.to_str().unwrap()
            );
        }

        for file in files.iter_mut() {
            file.load().expect("Failed to load file");
            if save_files {
                file.save().expect("Failed to save file");
            }
        }
        let (qasm, json): (Vec<_>, Vec<_>) = files
            .into_iter()
            .filter(|f| f.valid_pattern())
            .map(|f| (f.qasm.contents.unwrap(), f.json.contents.unwrap()))
            .unzip();

        let mut qasm_bin_file = fs::File::create(self.circuit_folder().join("qasm.bin")).unwrap();
        let mut json_bin_file = fs::File::create(self.circuit_folder().join("json.bin")).unwrap();
        rmp_serde::encode::write(&mut qasm_bin_file, &qasm).unwrap();
        rmp_serde::encode::write(&mut json_bin_file, &json).unwrap();
        qasm.len()
    }
}
