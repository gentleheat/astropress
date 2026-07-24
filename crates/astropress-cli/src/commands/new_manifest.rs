use crate::cli_config::env::PackageManifest;

/// Pin the astropress dependency the scaffolded project will install.
///
/// The scoped name is the one every generated source imports (`astro.config.mjs`,
/// `src/middleware.ts`, `src/astropress/*`). The embedded template ships it as
/// `workspace:*`, which only resolves for the in-repo examples under `examples/`,
/// so it must be rewritten here — otherwise `bun install` fails outright for any
/// project scaffolded outside the monorepo, on the user's very first command.
///
/// The bare `astropress` key is deliberately not emitted: the package publishes
/// no `bin`, no scaffolded file imports it, and the integration already aliases
/// `astropress` -> `@astropress-diy/astropress` in Vite for user-authored code
/// (see `packages/astropress/src/integration-host-config.ts`).
pub(super) fn pin_astropress_dependency(
    manifest: &mut PackageManifest,
    use_local_package: bool,
    published_version: &str,
) {
    let specifier = if use_local_package {
        format!(
            "file:{}",
            crate::repo_root().join("packages").join("astropress").display()
        )
    } else {
        format!("^{published_version}")
    };
    manifest
        .dependencies
        .insert("@astropress-diy/astropress".into(), specifier);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A manifest in the state the embedded template leaves it: carrying the
    /// `workspace:*` specifier that only resolves inside the monorepo.
    fn template_manifest() -> PackageManifest {
        PackageManifest {
            name: "demo".into(),
            private: true,
            package_type: Some("module".into()),
            scripts: BTreeMap::new(),
            dependencies: BTreeMap::from([(
                "@astropress-diy/astropress".to_string(),
                "workspace:*".to_string(),
            )]),
            dev_dependencies: BTreeMap::new(),
        }
    }

    #[test]
    fn local_package_replaces_the_workspace_specifier_with_a_file_path() {
        let mut manifest = template_manifest();
        pin_astropress_dependency(&mut manifest, true, "0.0.3");

        let pinned = &manifest.dependencies["@astropress-diy/astropress"];
        assert!(pinned.starts_with("file:"), "expected a file: path, got `{pinned}`");
        assert!(pinned.ends_with("packages/astropress"), "should point at the local package, got `{pinned}`");
    }

    #[test]
    fn published_package_pins_the_supplied_semver_range() {
        let mut manifest = template_manifest();
        pin_astropress_dependency(&mut manifest, false, "9.9.9");

        assert_eq!(manifest.dependencies["@astropress-diy/astropress"], "^9.9.9");
    }

    #[test]
    fn never_emits_the_bare_astropress_alias() {
        let mut manifest = template_manifest();
        pin_astropress_dependency(&mut manifest, true, "0.0.3");

        assert!(
            !manifest.dependencies.contains_key("astropress"),
            "the bare alias resolves to nothing the scaffold imports"
        );
    }
}
