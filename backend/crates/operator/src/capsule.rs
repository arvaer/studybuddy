//! The program the operator runs (19c): `backend/capsules/learning.capsule`
//! and its environment, both templates with one hole, `{{workspace}}`,
//! filled with the workspace id before compiling. The hole is in the
//! capsule's scope paths and in the environment's grants, so the path every
//! effect names is the host's; the model never chooses where an activity
//! goes, and a capsule compiled for one workspace is refused by another's
//! environment before any provider is asked.
//!
//! The templates are read at build time, so the repository's capsule source
//! is what runs, and `check` compiles them at startup so a broken program
//! fails the process, not the first learner.

use capsule_corp::sdk::Capsule;
use uuid::Uuid;

pub use capsule_corp::sdk::CompileError;

pub const CAPSULE_TEMPLATE: &str = include_str!("../../../capsules/learning.capsule");
pub const ENVIRONMENT_TEMPLATE: &str = include_str!("../../../capsules/learning.environment");

/// The one hole in both templates.
pub const HOLE: &str = "{{workspace}}";

/// The capsule source for `workspace`.
pub fn source(workspace: Uuid) -> String {
    CAPSULE_TEMPLATE.replace(HOLE, &workspace.to_string())
}

/// The environment source for `workspace`.
pub fn environment(workspace: Uuid) -> String {
    ENVIRONMENT_TEMPLATE.replace(HOLE, &workspace.to_string())
}

/// The capsule compiled for `workspace`, ready to instantiate.
pub fn compile(workspace: Uuid) -> Result<Capsule, CompileError> {
    Capsule::compile(&source(workspace))
}

/// Compile the template once, for startup: its name and the definition
/// address it has for a nil workspace, so the log pins what source this
/// process runs. The hole sits inside string literals, so an unfilled
/// template compiles too; what refuses a misfilled one is the environment,
/// whose grants name the real workspace.
pub fn check() -> Result<(String, String), CompileError> {
    let capsule = compile(Uuid::nil())?;
    Ok((
        capsule.name().to_string(),
        capsule.definition().address().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_templates_compile_once_filled_and_never_unfilled() {
        let (name, address) = check().unwrap();
        assert_eq!(name, "learning-operator");
        assert!(address.starts_with("sha256:"), "{address}");
        let ws = Uuid::new_v4();
        assert!(!source(ws).contains(HOLE) && !environment(ws).contains(HOLE));
        assert!(source(ws).contains(&format!("\"workspaces/{ws}/*\"")));
        assert!(environment(ws).contains(&format!(":scope \"workspaces/{ws}/*\"")));
    }
}
