use std::path::Path;
use std::process::{Command, Stdio};

/// Initialise a git repository for a freshly scaffolded project.
///
/// The scaffold emits `.gitignore` and `.github/workflows`, so a repository is
/// the expected end state, and the generated lefthook hooks — including the one
/// that refuses to commit `.env`, which now holds real admin and session
/// secrets — only get installed when `bun install` runs `prepare` inside a work
/// tree.
///
/// Returns whether a repository was actually created. Scaffolding *into* an
/// existing work tree leaves it alone: nesting a repository inside another
/// would be worse than doing nothing. A missing or failing `git` is not fatal
/// either — the scaffolded `prepare` script tolerates the hooks not installing.
pub(super) fn init_git_repository(project_dir: &Path) -> bool {
    if is_inside_work_tree(project_dir) {
        return false;
    }

    run_git_quietly(project_dir, &["init"])
}

fn is_inside_work_tree(project_dir: &Path) -> bool {
    run_git_quietly(project_dir, &["rev-parse", "--is-inside-work-tree"])
}

/// Runs `git` in `project_dir`, discarding its output, and reports whether it
/// exited successfully. A `git` that is missing or cannot be spawned reports
/// `false` rather than propagating an error.
fn run_git_quietly(project_dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(project_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A fresh directory under the OS temp dir, which is outside any work tree.
    fn unique_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("astropress-{label}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn initialises_a_repository_outside_any_work_tree() {
        let dir = unique_dir("git-init");

        assert!(init_git_repository(&dir), "should report that it created a repository");
        assert!(dir.join(".git").exists(), "a work tree should exist afterwards");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn leaves_an_existing_work_tree_alone() {
        let outer = unique_dir("git-nested");
        assert!(init_git_repository(&outer), "failed to create the outer repository");

        let inner = outer.join("child");
        fs::create_dir_all(&inner).unwrap();

        assert!(!init_git_repository(&inner), "must not initialise inside an existing work tree");
        assert!(!inner.join(".git").exists(), "a second repository must not be nested");

        fs::remove_dir_all(&outer).ok();
    }
}
