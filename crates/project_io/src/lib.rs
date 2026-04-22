//! Save, load, and export. Versioned JSON for project files;
//! mesh exports (STL, 3MF) land alongside in dedicated modules.

use std::fs;
use std::path::Path;

mod error;
pub mod schema;

use roncad_geometry::Project;

pub use error::{ProjectIoError, Result};
pub use schema::ProjectFile;

pub fn project_to_json(project: &Project) -> Result<String> {
    let file = ProjectFile::from_project(project)?;
    serde_json::to_string_pretty(&file).map_err(ProjectIoError::from)
}

pub fn project_from_json(json: &str) -> Result<Project> {
    let file: ProjectFile = serde_json::from_str(json)?;
    file.into_project()
}

pub fn save_project(project: &Project, path: impl AsRef<Path>) -> Result<()> {
    fs::write(path, project_to_json(project)?)?;
    Ok(())
}

pub fn load_project(path: impl AsRef<Path>) -> Result<Project> {
    project_from_json(&fs::read_to_string(path)?)
}
