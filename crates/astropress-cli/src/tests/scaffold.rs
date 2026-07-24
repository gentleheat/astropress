//! Scaffold, env-config, services, and sync tests.
//! Extracted from `mod.rs` to keep that file under the 300-line arch-lint threshold.

use super::*;

#[test]
fn scaffolds_new_project_from_example() {
    let root = temp_dir("new");
    let project_dir = root.join("demo");
    // Pass explicit app_host to skip the interactive hosting prompt
    scaffold_new_project(&project_dir, true, LocalProvider::Supabase, Some(AppHost::Vercel), Some(DataServices::Supabase), crate::commands::new::ScaffoldOptions { analytics_flag: None, ab_testing_flag: None, heatmap_flag: None, enable_api_flag: false, yes_defaults_flag: false }).unwrap();

    let package_json = fs::read_to_string(project_dir.join("package.json")).unwrap();
    assert!(package_json.contains("\"name\": \"demo\""));
    assert!(package_json.contains("\"@astropress-diy/astropress\": \"file:"));
    let env_contents = fs::read_to_string(project_dir.join(".env")).unwrap();
    assert!(env_contents.contains("ASTROPRESS_CONTENT_SERVICES=supabase"));
    assert!(!env_contents.contains("ASTROPRESS_LOCAL_PROVIDER="));
    assert!(!env_contents.contains("ASTROPRESS_DEPLOY_TARGET="));
    assert!(env_contents.contains(&format!(
        "ADMIN_DB_PATH={}",
        default_admin_db_relative_path(LocalProvider::Supabase)
    )));
    // Passwords are now passphrases: word+char-word+char-word+char-word+char
    assert!(env_contents.contains("ADMIN_PASSWORD="));
    assert!(env_contents.contains("EDITOR_PASSWORD="));
    assert!(env_contents.contains("ADMIN_BOOTSTRAP_DISABLED=0"));
    let admin_pw = env_contents.lines()
        .find(|l| l.starts_with("ADMIN_PASSWORD="))
        .and_then(|l| l.strip_prefix("ADMIN_PASSWORD="))
        .unwrap_or("");
    // passphrase has exactly 3 hyphens separating 4 word+char segments
    assert_eq!(admin_pw.chars().filter(|&c| c == '-').count(), 3, "ADMIN_PASSWORD should have 3 hyphens: {admin_pw}");
    assert!(env_contents.contains("SESSION_SECRET="));
    let env_example = fs::read_to_string(project_dir.join(".env.example")).unwrap();
    assert!(env_example.contains("SUPABASE_URL=https://your-project.supabase.co"));
    assert!(env_example.contains(
        "ASTROPRESS_SERVICE_ORIGIN=https://your-project.supabase.co/functions/v1/astropress"
    ));
    assert!(!env_example.contains("ASTROPRESS_HOSTED_PROVIDER="));
    assert!(!env_example.contains("SUPABASE_ANON_KEY="));
    assert!(env_example.contains("SUPABASE_SERVICE_ROLE_KEY=replace-me"));
    assert!(env_example.contains(
        "ADMIN_PASSWORD=replace-with-a-generated-local-admin-password"
    ));
    assert!(env_example.contains(
        "SESSION_SECRET=replace-with-a-long-random-session-secret"
    ));
    assert!(project_dir.join(".data/.gitkeep").exists());
    assert!(project_dir.join("src/pages/index.astro").exists());
    assert!(project_dir.join("DEPLOY.md").exists());
    assert!(project_dir.join(".github/workflows/deploy-astropress.yml").exists());
    assert!(package_json.contains("\"doctor:strict\": \"astropress doctor --strict\""));
    assert!(package_json.contains("\"deploy:vercel\":"));

    // Templates carrying a `.tpl` suffix are emitted with the suffix stripped
    // so biome 2 doesn't trip on a nested-root config in the parent monorepo.
    // Catches mutants on the suffix detection (`ends_with(".tpl")`) and the
    // slice arithmetic (`s.len() - 4`) in scaffold.rs::write_embedded_dir.
    assert!(project_dir.join("biome.json").exists(), "biome.json should be emitted with the .tpl suffix stripped");
    assert!(!project_dir.join("biome.json.tpl").exists(), "biome.json.tpl should not appear in scaffold output");
}

#[test]
fn preserves_existing_data_dir_when_setting_sqlite_defaults() {
    let root = temp_dir("env");
    fs::write(root.join(".data-existing"), "").unwrap();

    ensure_local_provider_defaults(&root).unwrap();

    assert!(root.join(".data/.gitkeep").exists());
}

#[test]
fn reads_provider_and_db_path_from_env() {
    let root = temp_dir("env-config");
    fs::write(
        root.join(".env"),
        "ASTROPRESS_LOCAL_PROVIDER=supabase\nADMIN_DB_PATH=.data/custom-supabase.sqlite\n",
    )
    .unwrap();

    let env_values = read_env_file(&root).unwrap();
    assert_eq!(
        env_values.get("ASTROPRESS_LOCAL_PROVIDER"),
        Some(&"supabase".to_string())
    );
    let project_env = load_project_env_contract(&root).unwrap();
    assert_eq!(project_env.local_provider, "supabase");
    assert_eq!(project_env.admin_db_path, ".data/custom-supabase.sqlite");
    assert_eq!(
        resolve_local_provider(&root, None).unwrap(),
        LocalProvider::Supabase
    );
    assert_eq!(
        resolve_admin_db_path(&root, LocalProvider::Supabase).unwrap(),
        ".data/custom-supabase.sqlite"
    );
}

#[test]
fn migrates_legacy_env_keys() {
    let root = temp_dir("config-migrate");
    fs::write(
        root.join(".env"),
        "ASTROPRESS_DATA_SERVICES=supabase\nSUPABASE_URL=https://demo.supabase.co\nASTROPRESS_DEPLOY_TARGET=cloudflare\nASTROPRESS_HOSTED_PROVIDER=supabase\n",
    )
    .unwrap();

    let changed = migrate_project_config(&root, false).unwrap();
    assert_eq!(changed, 1);

    let env_values = read_env_file(&root).unwrap();
    assert_eq!(
        env_values.get("ASTROPRESS_CONTENT_SERVICES"),
        Some(&"supabase".to_string())
    );
    assert!(!env_values.contains_key("ASTROPRESS_DATA_SERVICES"));
    assert!(!env_values.contains_key("ASTROPRESS_HOSTED_PROVIDER"));
    assert!(!env_values.contains_key("ASTROPRESS_DEPLOY_TARGET"));
    assert_eq!(
        env_values.get("ASTROPRESS_APP_HOST"),
        Some(&"cloudflare-pages".to_string())
    );
    assert_eq!(
        env_values.get("ASTROPRESS_SERVICE_ORIGIN"),
        Some(&"https://demo.supabase.co/functions/v1/astropress".to_string())
    );
}

#[test]
fn bootstraps_and_verifies_content_services() {
    let root = temp_dir("services");
    fs::write(
        root.join(".env"),
        "ASTROPRESS_CONTENT_SERVICES=supabase\nSUPABASE_URL=https://demo.supabase.co\nSUPABASE_ANON_KEY=anon\nSUPABASE_SERVICE_ROLE_KEY=service\nASTROPRESS_SERVICE_ORIGIN=https://demo.supabase.co/functions/v1/astropress\n",
    )
    .unwrap();

    bootstrap_content_services(&root).unwrap();
    let report = verify_content_services(&root).unwrap();
    assert_eq!(report.support_level, "configured");
    assert!(root.join(".astropress/services/supabase.json").exists());
}

#[test]
fn project_runtime_plan_exposes_local_runtime_selection() {
    let root = temp_dir("project-runtime");
    fs::write(
        root.join(".env"),
        "ASTROPRESS_RUNTIME_MODE=local\nASTROPRESS_LOCAL_PROVIDER=supabase\nADMIN_DB_PATH=.data/local-supabase.sqlite\n",
    )
    .unwrap();

    let plan = load_project_runtime_plan(&root, None, None, None).unwrap();
    assert_eq!(plan.mode, "local");
    assert_eq!(plan.env.local_provider, "supabase");
    assert_eq!(plan.env.admin_db_path, ".data/local-supabase.sqlite");
    assert_eq!(plan.adapter.capabilities.name, "supabase");
}

#[test]
fn explicit_provider_overrides_env_provider() {
    let root = temp_dir("env-provider-override");
    fs::write(root.join(".env"), "ASTROPRESS_LOCAL_PROVIDER=supabase\n").unwrap();

    assert_eq!(
        resolve_local_provider(&root, Some(LocalProvider::Supabase)).unwrap(),
        LocalProvider::Supabase
    );
}

#[test]
fn deploy_target_defaults_follow_local_provider() {
    let root = temp_dir("deploy-target");
    fs::write(root.join(".env"), "ASTROPRESS_LOCAL_PROVIDER=supabase\n").unwrap();

    assert_eq!(resolve_deploy_target(&root, None).unwrap(), "vercel");
    assert_eq!(
        resolve_deploy_target(&root, Some("cloudflare")).unwrap(),
        "cloudflare"
    );
}

#[test]
fn deploy_target_prefers_explicit_env_target() {
    let root = temp_dir("deploy-target-env");
    fs::write(
        root.join(".env"),
        "ASTROPRESS_LOCAL_PROVIDER=sqlite\nASTROPRESS_DEPLOY_TARGET=netlify\n",
    )
    .unwrap();

    assert_eq!(resolve_deploy_target(&root, None).unwrap(), "netlify");
}

#[test]
fn stages_wordpress_imports() {
    let root = temp_dir("import");
    let project_dir = root.join("project");
    fs::create_dir_all(&project_dir).unwrap();
    let source = root.join("export.xml");
    fs::write(&source, "<rss></rss>").unwrap();

    stage_wordpress_import(&project_dir, Some(&source), None, None, None, None, None, true, false, false, cli_config::args::CrawlMode::None).unwrap();
    assert!(project_dir.join(".astropress/import/wordpress-source.xml").exists());
    let manifest = fs::read_to_string(project_dir.join(".astropress/import/manifest.json")).unwrap();
    assert!(manifest.contains("\"inventory_file\": \"wordpress.inventory.json\""));
    assert!(manifest.contains("\"report_file\": \"wordpress.report.json\""));
    assert!(manifest.contains("\"content_file\":"));
    let report = fs::read_to_string(project_dir.join(".astropress/import/wordpress.report.json")).unwrap();
    assert!(report.contains("\"status\":"));
    assert!(report.contains("\"imported_records\": 0"));
    assert!(report.contains("\"downloaded_media\": 0"));
}

#[test]
fn exports_and_imports_project_snapshots() {
    let root = temp_dir("sync");
    let project_dir = root.join("project");
    fs::create_dir_all(project_dir.join("src")).unwrap();
    fs::write(project_dir.join("package.json"), "{\"name\":\"demo\",\"scripts\":{}}").unwrap();
    fs::write(project_dir.join("src/index.txt"), "hello").unwrap();

    let snapshot_dir = root.join("snapshot");
    export_project_snapshot(&project_dir, Some(&snapshot_dir)).unwrap();
    assert!(snapshot_dir.join("package.json").exists());
    assert!(snapshot_dir.join("src/index.txt").exists());

    fs::write(snapshot_dir.join("src/index.txt"), "updated").unwrap();
    import_project_snapshot(&project_dir, &snapshot_dir).unwrap();
    assert_eq!(
        fs::read_to_string(project_dir.join("src/index.txt")).unwrap(),
        "updated"
    );
}

#[test]
fn migrate_project_config_updates_package_json_scripts() {
    use crate::cli_config::env::read_package_manifest;
    let root = temp_dir("config-migrate-pkg");
    // Write a package.json with a legacy --data-services flag
    fs::write(
        root.join("package.json"),
        r#"{"name":"test","private":false,"scripts":{"dev":"astropress dev --data-services supabase"},"dependencies":{}}"#,
    ).unwrap();

    let changed = migrate_project_config(&root, false).unwrap();
    assert!(changed >= 1, "expected >= 1 changes for package.json migration, got {changed}");

    let manifest = read_package_manifest(&root).unwrap();
    assert!(manifest.scripts["dev"].contains("--content-services"),
        "expected --content-services in migrated script, got: {}", manifest.scripts["dev"]);
}

#[test]
fn migrate_project_config_dry_run_does_not_write_package_json() {
    let root = temp_dir("config-dry-run-pkg");
    let original_pkg = r#"{"name":"test","private":false,"scripts":{"dev":"astropress dev --data-services supabase"},"dependencies":{}}"#;
    fs::write(root.join("package.json"), original_pkg).unwrap();

    let changed = migrate_project_config(&root, true).unwrap();
    assert!(changed >= 1, "expected >= 1 changes reported even in dry run, got {changed}");

    // dry_run=true must not modify the file
    let on_disk = fs::read_to_string(root.join("package.json")).unwrap();
    assert!(on_disk.contains("--data-services"),
        "dry run must not rewrite package.json, got: {on_disk}");
}
