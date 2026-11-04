//! Build-from-source for compose services: fetch the source (git clone or a
//! path on the server), build each configured image tagged with the revision
//! and `latest`, and push both tags to the image's registry.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::images::image_repo;
use super::runner::{CommandRunner, CommandSpec, DeployLog};
use super::DbLog;
use crate::db::{service_source_type, ServiceBuildImage, ServiceDeployConfig};

/// Longest a single image build or push may take.
const BUILD_TIMEOUT: Duration = Duration::from_secs(45 * 60);
const GIT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Fetched source tree and the revision it represents.
#[derive(Debug, Clone)]
pub struct Source {
    pub root: PathBuf,
    /// Directory to delete when the deployment ends (a fresh clone).
    pub cleanup: Option<PathBuf>,
    pub commit_sha: Option<String>,
    pub commit_message: Option<String>,
    /// Tag applied next to `latest`: the short commit sha, the revision file's
    /// content, or a UTC timestamp.
    pub tag: String,
}

/// Replace `${COMMIT_SHA}`, `${SHORT_SHA}` and `${TAG}` in a build arg value.
pub fn substitute_build_vars(value: &str, source: &Source) -> String {
    let sha = source.commit_sha.clone().unwrap_or_default();
    let short = sha.chars().take(7).collect::<String>();
    value
        .replace("${COMMIT_SHA}", &sha)
        .replace("${SHORT_SHA}", &short)
        .replace("${TAG}", &source.tag)
}

/// A path relative to the source root, or an absolute path as given.
pub fn resolve_path(root: &Path, p: &str) -> PathBuf {
    let p = p.trim();
    if p.is_empty() || p == "." {
        return root.to_path_buf();
    }
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

/// `docker build` for one image, tagged `<repo>:<tag>` and `<repo>:latest`.
pub fn build_command(source: &Source, img: &ServiceBuildImage) -> CommandSpec {
    let repo = image_repo(img.image.trim());
    let mut args: Vec<String> = vec![
        "build".into(),
        "--progress=plain".into(),
        "-f".into(),
        resolve_path(&source.root, &img.dockerfile)
            .to_string_lossy()
            .into_owned(),
        "-t".into(),
        format!("{}:{}", repo, source.tag),
        "-t".into(),
        format!("{}:latest", repo),
    ];
    if let Some(target) = img.target.as_deref().filter(|t| !t.trim().is_empty()) {
        args.push("--target".into());
        args.push(target.trim().to_string());
    }
    for (k, v) in &img.build_args {
        args.push("--build-arg".into());
        args.push(format!("{}={}", k, substitute_build_vars(v, source)));
    }
    args.push(
        resolve_path(&source.root, &img.context)
            .to_string_lossy()
            .into_owned(),
    );
    CommandSpec::args("docker", args)
        .cwd(&source.root)
        .timeout(BUILD_TIMEOUT)
        .streamed()
}

/// Registry host of an image reference (`docker.io` for Docker Hub names).
pub fn registry_host(image: &str) -> String {
    let repo = image_repo(image);
    match repo.split_once('/') {
        Some((first, _)) if first.contains('.') || first.contains(':') || first == "localhost" => {
            first.to_string()
        }
        _ => "docker.io".to_string(),
    }
}

/// Canonical form of a git remote for matching webhooks to sources:
/// `host/owner/repo`, lowercase, without scheme, credentials or `.git`.
pub fn normalize_git_url(url: &str) -> String {
    let mut u = url.trim().to_lowercase();
    for prefix in ["https://", "http://", "ssh://", "git://"] {
        if let Some(rest) = u.strip_prefix(prefix) {
            u = rest.to_string();
            break;
        }
    }
    if let Some(at) = u.find('@') {
        // Credentials (https) or the `git@` user (scp-like ssh).
        if !u[..at].contains('/') {
            u = u[at + 1..].to_string();
        }
    }
    // scp-like `host:owner/repo` -> `host/owner/repo` (but keep `host:port/...`).
    if let Some(colon) = u.find(':') {
        let after = &u[colon + 1..];
        let is_port = after
            .split('/')
            .next()
            .map(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(false);
        if is_port {
            let slash = after.find('/').map(|i| colon + 1 + i).unwrap_or(u.len());
            u = format!("{}{}", &u[..colon], &u[slash..]);
        } else {
            u = format!("{}/{}", &u[..colon], after);
        }
    }
    let u = u.trim_end_matches('/');
    u.strip_suffix(".git").unwrap_or(u).to_string()
}

/// Compose services to pull before `up`: every service except those whose
/// image this deployment just built (pulling those could replace the fresh
/// local build with an older registry copy when pushing is off). `None` means
/// pull everything.
pub fn services_to_pull(compose: &str, built_repos: &BTreeSet<String>) -> Option<Vec<String>> {
    if built_repos.is_empty() {
        return None;
    }
    let yaml: serde_yaml::Value = serde_yaml::from_str(compose).ok()?;
    let services = yaml.get("services")?.as_mapping()?;
    let mut out = vec![];
    for (name, def) in services {
        let Some(name) = name.as_str() else { continue };
        let image = def.get("image").and_then(|v| v.as_str());
        match image {
            Some(img) if built_repos.contains(image_repo(img)) => {}
            // Services with only a `build:` key have nothing to pull either.
            None => {}
            Some(_) => out.push(name.to_string()),
        }
    }
    Some(out)
}

/// Validate the build part of a deploy config. Returns a user-facing message.
pub fn validate_build_config(cfg: &ServiceDeployConfig) -> Result<(), String> {
    match cfg.source_type.as_str() {
        service_source_type::NONE => {}
        service_source_type::GIT => {
            let url = cfg.git_url.as_deref().unwrap_or("").trim();
            if url.is_empty() {
                return Err("A git source needs a repository URL".into());
            }
            if url.starts_with('-') || url.chars().any(|c| c.is_whitespace()) {
                return Err("Invalid repository URL".into());
            }
            validate_ref(&cfg.branch).map_err(|e| format!("Branch: {}", e))?;
        }
        service_source_type::PATH => {
            let p = cfg.source_path.as_deref().unwrap_or("").trim();
            if !Path::new(p).is_absolute() {
                return Err("A server path source needs an absolute path".into());
            }
        }
        other => return Err(format!("Unknown source type '{}'", other)),
    }
    if let Some(rf) = cfg
        .revision_file
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        if rf.contains('\n') {
            return Err("Invalid revision file path".into());
        }
    }

    let images = cfg.build_images();
    if cfg.source_type != service_source_type::NONE && images.is_empty() {
        return Err("Add at least one image to build".into());
    }
    let mut seen = BTreeSet::new();
    for img in &images {
        let image = img.image.trim();
        if image.is_empty() {
            return Err("Every image needs a target repository".into());
        }
        if image.contains('@') {
            return Err(format!("{}: use a repository without a digest", image));
        }
        let valid = image
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-/:".contains(c));
        if !valid || image.starts_with('-') {
            return Err(format!(
                "{}: image names may only use lowercase letters, digits and . _ - / :",
                image
            ));
        }
        if !seen.insert(image_repo(image).to_string()) {
            return Err(format!("{} is listed twice", image_repo(image)));
        }
        for (field, v) in [("Dockerfile", &img.dockerfile), ("Context", &img.context)] {
            if v.contains('\n') || v.trim_start().starts_with('-') {
                return Err(format!("{}: invalid {} path", image, field));
            }
        }
        for k in img.build_args.keys() {
            let ok = !k.is_empty()
                && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !k.chars().next().unwrap().is_ascii_digit();
            if !ok {
                return Err(format!("{}: invalid build arg name '{}'", image, k));
            }
        }
        if let Some(t) = &img.target {
            if t.contains(char::is_whitespace) || t.starts_with('-') {
                return Err(format!("{}: invalid build target", image));
            }
        }
    }
    Ok(())
}

/// A branch or ref name that is safe to pass to git.
pub fn validate_ref(r: &str) -> Result<(), String> {
    let r = r.trim();
    if r.is_empty() {
        return Err("must not be empty".into());
    }
    if r.starts_with('-')
        || r.contains("..")
        || r.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("invalid characters".into());
    }
    Ok(())
}

fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

/// Read HEAD's sha and subject from a git checkout. `None` if not a repo.
async fn git_head(runner: &dyn CommandRunner, dir: &Path) -> Option<(String, String)> {
    let sha = runner
        .run(
            &CommandSpec::new("git", &["rev-parse", "HEAD"]).cwd(dir),
            None,
        )
        .await
        .ok()
        .filter(|o| o.success)?
        .stdout
        .trim()
        .to_string();
    if sha.is_empty() {
        return None;
    }
    let msg = runner
        .run(
            &CommandSpec::new("git", &["log", "-1", "--format=%s"]).cwd(dir),
            None,
        )
        .await
        .ok()
        .filter(|o| o.success)
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default();
    Some((sha, msg))
}

/// Clone (git source) or locate (path source) the source tree.
pub async fn fetch_source(
    env: &super::DeployEnv,
    cfg: &ServiceDeployConfig,
    service_name: &str,
    deployment_id: &str,
    log: &DbLog,
) -> Result<Source> {
    let db = &env.db;
    let data_dir = env.data_dir.as_path();
    let encryption_key = env.encryption_key.as_ref();
    let runner = env.runner.as_ref();
    match cfg.source_type.as_str() {
        service_source_type::GIT => {
            let git_url = cfg.git_url.clone().unwrap_or_default();
            let dest = data_dir
                .join("service-builds")
                .join(service_name)
                .join(deployment_id);
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let _ = tokio::fs::remove_dir_all(&dest).await;

            let auth = crate::engine::authenticated_git_url(
                db,
                &git_url,
                cfg.github_app_installation_id.as_deref(),
                cfg.git_provider_id.as_deref(),
                encryption_key,
            )
            .await
            .context("Failed to prepare git credentials")?;
            if let Some(secret) = &auth.secret {
                log.add_secret(secret);
            }

            let mut spec = CommandSpec::args(
                "git",
                [
                    "clone".to_string(),
                    "--depth".into(),
                    "1".into(),
                    "--branch".into(),
                    cfg.branch.trim().to_string(),
                    "--".into(),
                    auth.url.clone(),
                    dest.to_string_lossy().into_owned(),
                ],
            )
            .env("GIT_TERMINAL_PROMPT", "0")
            .timeout(GIT_TIMEOUT)
            .streamed();

            // SSH remotes use the global SSH key, like app deploys.
            let mut key_file: Option<PathBuf> = None;
            if crate::engine::is_ssh_url(&git_url) {
                let key: Option<crate::db::SshKey> = sqlx::query_as(
                    "SELECT * FROM ssh_keys WHERE is_global = 1 ORDER BY created_at DESC LIMIT 1",
                )
                .fetch_optional(db)
                .await?;
                if let Some(key) = key {
                    let pem = crate::crypto::decrypt_if_encrypted(&key.private_key, encryption_key)
                        .unwrap_or(key.private_key);
                    let path = std::env::temp_dir()
                        .join(format!("rivetr-svc-ssh-{}", uuid::Uuid::new_v4()));
                    tokio::fs::write(&path, pem).await?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                            .await?;
                    }
                    spec = spec.env(
                        "GIT_SSH_COMMAND",
                        &format!(
                            "ssh -i {} -o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null",
                            path.display()
                        ),
                    );
                    key_file = Some(path);
                }
            }

            log.info(&format!(
                "Cloning {} (branch {})",
                git_url,
                cfg.branch.trim()
            ))
            .await;
            let out = runner.run(&spec, Some(log)).await;
            if let Some(k) = key_file {
                let _ = tokio::fs::remove_file(k).await;
            }
            let out = out?;
            if !out.success {
                let _ = tokio::fs::remove_dir_all(&dest).await;
                anyhow::bail!("git clone failed: {}", log_safe(log, &out.error_tail()));
            }

            let (sha, msg) = git_head(runner, &dest)
                .await
                .context("Could not read the cloned commit")?;
            log.info(&format!("Checked out {} {}", short(&sha), msg))
                .await;
            Ok(Source {
                root: dest.clone(),
                cleanup: Some(dest),
                tag: short(&sha),
                commit_sha: Some(sha),
                commit_message: Some(msg).filter(|m| !m.is_empty()),
            })
        }
        service_source_type::PATH => {
            let root = PathBuf::from(cfg.source_path.clone().unwrap_or_default().trim());
            if !tokio::fs::metadata(&root)
                .await
                .map(|m| m.is_dir())
                .unwrap_or(false)
            {
                anyhow::bail!(
                    "Source path {} does not exist or is not a directory",
                    root.display()
                );
            }

            let mut commit_sha = None;
            let mut commit_message = None;
            let mut tag = None;
            if let Some(rf) = cfg
                .revision_file
                .as_deref()
                .filter(|s| !s.trim().is_empty())
            {
                let path = resolve_path(&root, rf);
                let rev = tokio::fs::read_to_string(&path)
                    .await
                    .with_context(|| format!("Could not read revision file {}", path.display()))?;
                let rev = rev.trim().to_string();
                if !rev.is_empty() {
                    tag = Some(sanitize_tag(&rev));
                    commit_sha = Some(rev);
                }
            } else if let Some((sha, msg)) = git_head(runner, &root).await {
                tag = Some(short(&sha));
                commit_sha = Some(sha);
                commit_message = Some(msg).filter(|m| !m.is_empty());
            }
            let tag = tag.unwrap_or_else(|| chrono::Utc::now().format("%Y%m%d%H%M%S").to_string());
            log.info(&format!(
                "Using source at {} (revision {})",
                root.display(),
                tag
            ))
            .await;
            Ok(Source {
                root,
                cleanup: None,
                commit_sha,
                commit_message,
                tag,
            })
        }
        other => anyhow::bail!("Service has no build source (source type '{}')", other),
    }
}

fn log_safe(log: &DbLog, s: &str) -> String {
    log.redact(s)
}

/// A docker tag: `[A-Za-z0-9_.-]`, at most 128 characters, not starting with `.` or `-`.
pub fn sanitize_tag(raw: &str) -> String {
    let mut t: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .take(128)
        .collect();
    while t.starts_with('.') || t.starts_with('-') {
        t.remove(0);
    }
    if t.is_empty() {
        chrono::Utc::now().format("%Y%m%d%H%M%S").to_string()
    } else {
        t
    }
}

/// Build every configured image, then push both tags. Returns the built repos.
pub async fn build_and_push(
    runner: &dyn CommandRunner,
    cfg: &ServiceDeployConfig,
    registry_password: Option<&str>,
    source: &Source,
    log: &DbLog,
) -> Result<BTreeSet<String>> {
    let images = cfg.build_images();
    let mut built = BTreeSet::new();

    for (i, img) in images.iter().enumerate() {
        let repo = image_repo(img.image.trim()).to_string();
        let label = img.name.clone().unwrap_or_else(|| repo.clone());
        log.info(&format!(
            "Building image {}/{}: {} -> {}:{}",
            i + 1,
            images.len(),
            label,
            repo,
            source.tag
        ))
        .await;
        let spec = build_command(source, img);
        log.info(&format!("$ {}", spec.display())).await;
        let out = runner.run(&spec, Some(log)).await?;
        if !out.success {
            anyhow::bail!("Building {} failed: {}", label, out.error_tail());
        }
        built.insert(repo);
    }

    if cfg.push_images == 0 {
        log.info("Pushing is off; images stay on this server only")
            .await;
        return Ok(built);
    }

    // Registry login with a throwaway docker config so credentials never land
    // in the service user's ~/.docker/config.json.
    let mut docker_config: Option<tempfile::TempDir> = None;
    if let (Some(user), Some(pass)) = (
        cfg.registry_username.as_deref().filter(|u| !u.is_empty()),
        registry_password.filter(|p| !p.is_empty()),
    ) {
        log.add_secret(pass);
        let dir = tempfile::tempdir().context("Failed to create a docker config dir")?;
        let mut hosts: Vec<String> = images.iter().map(|i| registry_host(&i.image)).collect();
        hosts.sort();
        hosts.dedup();
        for host in hosts {
            log.info(&format!("Logging in to {} as {}", host, user))
                .await;
            let spec = CommandSpec::args(
                "docker",
                ["login", host.as_str(), "-u", user, "--password-stdin"],
            )
            .env("DOCKER_CONFIG", &dir.path().to_string_lossy())
            .stdin(pass.to_string());
            let out = runner.run(&spec, Some(log)).await?;
            if !out.success {
                anyhow::bail!("Registry login to {} failed: {}", host, out.error_tail());
            }
        }
        docker_config = Some(dir);
    }

    for repo in &built {
        for tag in [source.tag.as_str(), "latest"] {
            let mut spec =
                CommandSpec::args("docker", ["push".to_string(), format!("{}:{}", repo, tag)])
                    .timeout(BUILD_TIMEOUT);
            if let Some(dir) = &docker_config {
                spec = spec.env("DOCKER_CONFIG", &dir.path().to_string_lossy());
            }
            log.info(&format!("$ {}", spec.display())).await;
            let out = runner.run(&spec, Some(log)).await?;
            if !out.success {
                anyhow::bail!("Pushing {}:{} failed: {}", repo, tag, out.error_tail());
            }
        }
    }
    log.info(&format!("Pushed {} image(s)", built.len())).await;
    Ok(built)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn source() -> Source {
        Source {
            root: PathBuf::from("/srv/src"),
            cleanup: None,
            commit_sha: Some("efb7da81234567890".into()),
            commit_message: None,
            tag: "efb7da8".into(),
        }
    }

    #[test]
    fn build_command_tags_revision_and_latest_and_substitutes_args() {
        let mut args = BTreeMap::new();
        args.insert(
            "BASE".to_string(),
            "localhost:5000/megasign-app:${TAG}".to_string(),
        );
        let img = ServiceBuildImage {
            name: None,
            image: "localhost:5000/megasign-worker".into(),
            dockerfile: "/opt/megasign/Dockerfile.worker".into(),
            context: "/opt/megasign".into(),
            build_args: args,
            target: Some("runtime".into()),
        };
        let spec = build_command(&source(), &img);
        assert_eq!(
            spec.display(),
            "docker build --progress=plain -f /opt/megasign/Dockerfile.worker \
             -t localhost:5000/megasign-worker:efb7da8 -t localhost:5000/megasign-worker:latest \
             --target runtime --build-arg BASE=localhost:5000/megasign-app:efb7da8 /opt/megasign"
        );
    }

    #[test]
    fn relative_paths_resolve_against_the_source_root() {
        let img = ServiceBuildImage {
            name: None,
            image: "localhost:5000/app:latest".into(),
            dockerfile: "infra/Dockerfile".into(),
            context: ".".into(),
            build_args: BTreeMap::new(),
            target: None,
        };
        let spec = build_command(&source(), &img);
        assert!(spec.display().contains("-f /srv/src/infra/Dockerfile"));
        assert!(spec.display().ends_with(" /srv/src"));
        // A tag on the configured image is replaced by the revision and latest.
        assert!(spec.display().contains("-t localhost:5000/app:efb7da8"));
    }

    #[test]
    fn registry_host_detection() {
        assert_eq!(registry_host("localhost:5000/app"), "localhost:5000");
        assert_eq!(registry_host("ghcr.io/org/app:1"), "ghcr.io");
        assert_eq!(registry_host("org/app"), "docker.io");
        assert_eq!(registry_host("app"), "docker.io");
    }

    #[test]
    fn git_urls_normalize_to_the_same_repo() {
        let want = "github.com/kwaminawhyte/mega-sign";
        for u in [
            "https://github.com/KwaminaWhyte/mega-sign.git",
            "https://github.com/KwaminaWhyte/mega-sign",
            "https://x-access-token:abc@github.com/KwaminaWhyte/mega-sign.git",
            "git@github.com:KwaminaWhyte/mega-sign.git",
            "ssh://git@github.com/KwaminaWhyte/mega-sign.git",
        ] {
            assert_eq!(normalize_git_url(u), want, "{}", u);
        }
        assert_eq!(
            normalize_git_url("ssh://git@git.example.com:2222/team/repo.git"),
            "git.example.com/team/repo"
        );
    }

    #[test]
    fn built_images_are_not_pulled() {
        let compose = "services:\n  api:\n    image: localhost:5000/megasign-app:latest\n  web:\n    image: localhost:5000/megasign-app:latest\n  worker:\n    image: localhost:5000/megasign-worker:latest\n  postgres:\n    image: postgres:18.3-alpine\n  local:\n    build: .\n";
        let mut built = BTreeSet::new();
        assert_eq!(services_to_pull(compose, &built), None);
        built.insert("localhost:5000/megasign-app".to_string());
        built.insert("localhost:5000/megasign-worker".to_string());
        assert_eq!(
            services_to_pull(compose, &built),
            Some(vec!["postgres".to_string()])
        );
    }

    #[test]
    fn tags_are_sanitized() {
        assert_eq!(sanitize_tag("efb7da8\n"), "efb7da8-");
        assert_eq!(sanitize_tag("v1.2/rc 1"), "v1.2-rc-1");
        assert_eq!(sanitize_tag("-x"), "x");
    }

    #[test]
    fn validation_rejects_bad_configs() {
        let mut cfg = ServiceDeployConfig::default_for("s");
        assert!(validate_build_config(&cfg).is_ok(), "pull-only is valid");

        cfg.source_type = "git".into();
        assert!(validate_build_config(&cfg).is_err(), "git needs a url");
        cfg.git_url = Some("https://github.com/o/r.git".into());
        assert!(validate_build_config(&cfg).is_err(), "needs an image");
        cfg.images = r#"[{"image":"localhost:5000/app"}]"#.into();
        assert!(validate_build_config(&cfg).is_ok());
        cfg.branch = "--upload-pack=x".into();
        assert!(validate_build_config(&cfg).is_err());
        cfg.branch = "main".into();
        cfg.images = r#"[{"image":"Localhost:5000/App"}]"#.into();
        assert!(validate_build_config(&cfg).is_err(), "uppercase");
        cfg.images = r#"[{"image":"a"},{"image":"a:latest"}]"#.into();
        assert!(validate_build_config(&cfg).is_err(), "duplicate repo");
        cfg.images = r#"[{"image":"a","build_args":{"1X":"y"}}]"#.into();
        assert!(validate_build_config(&cfg).is_err(), "bad arg name");

        cfg.source_type = "path".into();
        cfg.images = r#"[{"image":"a"}]"#.into();
        cfg.source_path = Some("relative/dir".into());
        assert!(validate_build_config(&cfg).is_err());
        cfg.source_path = Some("/opt/megasign/src".into());
        assert!(validate_build_config(&cfg).is_ok());
    }
}
