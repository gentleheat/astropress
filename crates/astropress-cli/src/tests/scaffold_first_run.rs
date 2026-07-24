//! What a freshly scaffolded project looks like before the user touches it.
//!
//! Every test here scaffolds into a temp directory — i.e. outside the astropress
//! workspace, the way a real user does — and asserts the project is immediately
//! usable. Extracted from `scaffold.rs` to keep that file under the 300-line
//! arch-lint threshold.

use super::*;

use crate::cli_config::env::read_package_manifest;

/// Non-interactive scaffold options: no optional features, no prompts.
fn default_scaffold_options() -> crate::commands::new::ScaffoldOptions {
    crate::commands::new::ScaffoldOptions {
        analytics_flag: None,
        ab_testing_flag: None,
        heatmap_flag: None,
        enable_api_flag: false,
        yes_defaults_flag: true,
    }
}

/// Scaffolds a github-pages/sqlite project into `<temp>/<label>/<name>`.
fn scaffold_demo_site(label: &str, name: &str, use_local_package: bool) -> (TestDir, PathBuf) {
    let root = temp_dir(label);
    let project_dir = root.join(name);
    scaffold_new_project(
        &project_dir,
        use_local_package,
        LocalProvider::Sqlite,
        Some(AppHost::GithubPages),
        Some(DataServices::None),
        default_scaffold_options(),
    )
    .unwrap();
    (root, project_dir)
}

#[test]
fn scaffold_pins_resolvable_specifiers_outside_the_monorepo() {
    // A `workspace:` specifier surviving into the manifest makes `bun install`
    // fail outright ("Workspace dependency `@astropress-diy/astropress` not
    // found"), which is every user's very first command after `astropress new`.
    let (_root, project_dir) = scaffold_demo_site("new-outside-workspace", "demo-site", true);

    let manifest = read_package_manifest(&project_dir).unwrap();
    for (name, spec) in manifest.dependencies.iter().chain(manifest.dev_dependencies.iter()) {
        assert!(
            !spec.starts_with("workspace:"),
            "dependency `{name}` kept an unresolvable workspace specifier: {spec}"
        );
    }

    // The scoped name is the one the generated sources actually import.
    let pinned = manifest
        .dependencies
        .get("@astropress-diy/astropress")
        .expect("scaffold must pin the package the generated sources import");
    assert!(
        pinned.starts_with("file:"),
        "--use-local-package should pin a file: path, got `{pinned}`"
    );

    // The bare alias is not a real dependency: the package publishes no `bin`,
    // no scaffolded file imports it, and Vite aliases it already.
    assert!(
        !manifest.dependencies.contains_key("astropress"),
        "the bare `astropress` alias should not be emitted as a dependency"
    );
}

#[test]
fn scaffold_pins_published_version_without_local_package() {
    let (_root, project_dir) = scaffold_demo_site("new-published-package", "demo-site", false);

    let manifest = read_package_manifest(&project_dir).unwrap();
    let pinned = manifest
        .dependencies
        .get("@astropress-diy/astropress")
        .expect("scaffold must pin the package the generated sources import");
    assert!(
        pinned.starts_with('^'),
        "--use-published-package should pin a semver range, got `{pinned}`"
    );
    assert!(
        !pinned.starts_with("workspace:") && !pinned.starts_with("file:"),
        "published scaffolds must not leak a workspace or local path: {pinned}"
    );
}

#[test]
fn scaffold_initialises_a_repository_and_keeps_prepare_non_fatal() {
    // The target is outside any work tree — the case where `lefthook install`
    // used to exit 128 and surface as a failed `prepare` on the first install.
    let (_root, project_dir) = scaffold_demo_site("new-git-init", "demo-site", true);

    assert!(
        project_dir.join(".git").exists(),
        "scaffolding outside a work tree should initialise a repository so the lefthook hooks install"
    );

    // Belt and braces: even where git is unavailable and no repository was
    // created, `prepare` must not fail the install.
    let manifest = read_package_manifest(&project_dir).unwrap();
    let prepare = manifest.scripts.get("prepare").expect("scaffold should emit a prepare script");
    assert!(
        prepare.contains("|| true"),
        "prepare must tolerate lefthook failing without a work tree, got `{prepare}`"
    );
}

#[test]
fn scaffold_does_not_nest_a_repository_inside_an_existing_one() {
    let root = temp_dir("new-git-nested");
    let status = std::process::Command::new("git")
        .arg("init")
        .current_dir(&*root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git should be available in the test environment");
    assert!(status.success(), "failed to create the outer test repository");

    let project_dir = root.join("nested-site");
    scaffold_new_project(
        &project_dir,
        true,
        LocalProvider::Sqlite,
        Some(AppHost::GithubPages),
        Some(DataServices::None),
        default_scaffold_options(),
    )
    .unwrap();

    assert!(
        !project_dir.join(".git").exists(),
        "scaffolding into an existing work tree must not nest a second repository"
    );
}
