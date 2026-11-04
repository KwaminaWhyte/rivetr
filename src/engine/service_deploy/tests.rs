//! Pipeline tests: a temp SQLite database and a recording mock runner.

use super::runner::mock::MockRunner;
use super::runner::CommandOutput;
use super::*;
use crate::db::parse_image_map;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) const COMPOSE: &str = r#"services:
  web:
    image: localhost:5000/app:latest
  db:
    image: postgres:18
"#;

pub(super) struct Harness {
    pub env: Arc<DeployEnv>,
    pub runner: Arc<MockRunner>,
    pub service_id: String,
    _dir: tempfile::TempDir,
}

pub(super) async fn harness() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let db = crate::db::init(dir.path()).await.unwrap();
    let service_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO services (id, name, compose_content, port, status) VALUES (?, 'shop', ?, 80, 'running')",
    )
    .bind(&service_id)
    .bind(COMPOSE)
    .execute(&db)
    .await
    .unwrap();
    let runner = Arc::new(MockRunner::new());
    let env = Arc::new(DeployEnv {
        db,
        data_dir: dir.path().to_path_buf(),
        encryption_key: None,
        runner: runner.clone(),
        routes: None,
    });
    Harness {
        env,
        runner,
        service_id,
        _dir: dir,
    }
}

/// Make the mock answer `compose ps` / `inspect` like a stack whose `web`
/// image changes from sha256:old to sha256:new once `up` has run.
pub(super) fn fake_stack(runner: &Arc<MockRunner>) {
    let ups = Arc::new(AtomicUsize::new(0));
    let u = ups.clone();
    runner.on_fn(" up -d", move |_| {
        u.fetch_add(1, Ordering::SeqCst);
        Some(CommandOutput::ok(""))
    });
    runner.on(
        " ps -a --format json",
        CommandOutput::ok(
            "{\"ID\":\"c1\",\"Name\":\"rivetr-svc-shop-web-1\",\"Image\":\"localhost:5000/app:latest\",\"Service\":\"web\",\"State\":\"running\"}\n\
             {\"ID\":\"c2\",\"Name\":\"rivetr-svc-shop-db-1\",\"Image\":\"postgres:18\",\"Service\":\"db\",\"State\":\"running\"}\n",
        ),
    );
    let u = ups.clone();
    runner.on_fn("docker inspect --format", move |_| {
        let web = if u.load(Ordering::SeqCst) > 0 {
            "sha256:new"
        } else {
            "sha256:old"
        };
        Some(CommandOutput::ok(&format!(
            "c1full {}\nc2full sha256:pg\n",
            web
        )))
    });
    runner.on(
        "docker image inspect",
        CommandOutput::ok(
            "sha256:old [\"localhost:5000/app@sha256:aaa\"]\n\
             sha256:new [\"localhost:5000/app@sha256:bbb\"]\n\
             sha256:pg [\"postgres@sha256:ccc\"]\n",
        ),
    );
}

pub(super) async fn deploy(h: &Harness, req: DeployRequest) -> ServiceDeployment {
    let (dep, _guard) = create_deployment(&h.env, &h.service_id, req)
        .await
        .expect("create deployment");
    run_deployment(&h.env, &dep.id).await.unwrap();
    sqlx::query_as("SELECT * FROM service_deployments WHERE id = ?")
        .bind(&dep.id)
        .fetch_one(&h.env.db)
        .await
        .unwrap()
}

pub(super) async fn logs(h: &Harness, dep: &ServiceDeployment) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT message FROM service_deployment_logs WHERE deployment_id = ? ORDER BY id",
    )
    .bind(&dep.id)
    .fetch_all(&h.env.db)
    .await
    .unwrap()
}

fn manual() -> DeployRequest {
    DeployRequest {
        trigger: "manual".into(),
        user_id: Some("u1".into()),
        user_name: Some("Ada".into()),
        ..Default::default()
    }
}

#[tokio::test]
async fn pull_only_deploy_records_digests_and_runs_in_order() {
    let h = harness().await;
    fake_stack(&h.runner);

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "succeeded", "{:?}", dep.error_message);
    assert_eq!(dep.triggered_by_name.as_deref(), Some("Ada"));
    assert!(dep.duration_ms.is_some());

    let pull = h
        .runner
        .position("compose -p rivetr-svc-shop pull")
        .unwrap();
    let up = h
        .runner
        .position("compose -p rivetr-svc-shop up -d --remove-orphans")
        .unwrap();
    assert!(pull < up);
    // Pull-only: nothing is built or pushed, and the stack is never taken down.
    assert!(!h.runner.ran("docker build"));
    assert!(!h.runner.ran("docker push"));
    assert!(!h.runner.ran(" down"));

    let before = parse_image_map(dep.images_before.as_deref());
    let after = dep.images_after_map();
    assert_eq!(
        before["web"].digest.as_deref(),
        Some("localhost:5000/app@sha256:aaa")
    );
    assert_eq!(
        after["web"].digest.as_deref(),
        Some("localhost:5000/app@sha256:bbb")
    );
    assert_eq!(after["db"].digest.as_deref(), Some("postgres@sha256:ccc"));

    let lines = logs(&h, &dep).await;
    assert!(
        lines.iter().any(|l| l.contains("Updated: web")),
        "{:?}",
        lines
    );

    // The compose file was written with Rivetr's network injection.
    let file = h.env.data_dir.join("services/shop/docker-compose.yml");
    let written = std::fs::read_to_string(file).unwrap();
    assert!(written.contains("rivetr"));
}

#[tokio::test]
async fn failed_pull_keeps_the_running_stack() {
    let h = harness().await;
    fake_stack(&h.runner);
    let dir = h.env.data_dir.join("services/shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("docker-compose.yml"), "# previous file\n").unwrap();
    h.runner
        .on(" pull", CommandOutput::failed(1, "registry unreachable"));

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "failed");
    assert!(dep.error_message.unwrap().contains("registry unreachable"));
    assert!(!h.runner.ran(" up -d"));
    assert_eq!(
        std::fs::read_to_string(dir.join("docker-compose.yml")).unwrap(),
        "# previous file\n"
    );
}

#[tokio::test]
async fn second_deploy_while_one_runs_is_rejected() {
    let h = harness().await;
    let (_dep, _guard) = create_deployment(&h.env, &h.service_id, manual())
        .await
        .unwrap();
    let second = create_deployment(&h.env, &h.service_id, manual()).await;
    assert!(matches!(second, Err(QueueError::Busy)));
}

#[tokio::test]
async fn unknown_service_is_not_found() {
    let h = harness().await;
    let r = create_deployment(&h.env, "nope", manual()).await;
    assert!(matches!(r, Err(QueueError::NotFound)));
}

#[test]
fn log_redacts_registered_secrets() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let h = harness().await;
        let log = DbLog::new(h.env.db.clone(), "x");
        log.add_secret("s3cr3t-token");
        assert_eq!(
            log.redact("https://x-access-token:s3cr3t-token@github.com/o/r"),
            "https://x-access-token:********@github.com/o/r"
        );
    });
}

pub(super) async fn set_config(h: &Harness, sql_set: &str) {
    sqlx::query("INSERT OR IGNORE INTO service_deploy_configs (service_id) VALUES (?)")
        .bind(&h.service_id)
        .execute(&h.env.db)
        .await
        .unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE service_deploy_configs SET {} WHERE service_id = ?",
        sql_set
    )))
    .bind(&h.service_id)
    .execute(&h.env.db)
    .await
    .unwrap();
}

#[tokio::test]
async fn git_source_builds_tags_pushes_then_deploys() {
    let h = harness().await;
    fake_stack(&h.runner);
    set_config(
        &h,
        "source_type = 'git', git_url = 'https://github.com/acme/shop.git', branch = 'main', \
         images = '[{\"image\":\"localhost:5000/app\",\"dockerfile\":\"infra/Dockerfile\"}]'",
    )
    .await;
    h.runner.on(
        "git rev-parse HEAD",
        CommandOutput::ok("0123456789abcdef0123456789abcdef01234567\n"),
    );
    h.runner
        .on("git log -1", CommandOutput::ok("Fix the cart\n"));

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "succeeded", "{:?}", dep.error_message);
    assert_eq!(dep.image_tag.as_deref(), Some("0123456"));
    assert_eq!(dep.commit_message.as_deref(), Some("Fix the cart"));

    let clone = h
        .runner
        .position("git clone --depth 1 --branch main -- https://github.com/acme/shop.git")
        .unwrap();
    let build = h
        .runner
        .position("docker build --progress=plain -f")
        .unwrap();
    let push_tag = h
        .runner
        .position("docker push localhost:5000/app:0123456")
        .unwrap();
    let push_latest = h
        .runner
        .position("docker push localhost:5000/app:latest")
        .unwrap();
    let up = h.runner.position(" up -d").unwrap();
    assert!(clone < build && build < push_tag && push_tag < push_latest && push_latest < up);
    let cmds = h.runner.commands();
    let build_cmd = &cmds[build];
    assert!(build_cmd.contains("-t localhost:5000/app:0123456 -t localhost:5000/app:latest"));
    assert!(build_cmd.contains("infra/Dockerfile"));
    // The freshly built web image is not pulled; postgres still is.
    let pull = cmds.iter().find(|c| c.contains(" pull")).unwrap();
    assert!(pull.ends_with("pull --ignore-pull-failures db"), "{}", pull);
    // The clone is removed after the deployment.
    assert!(!h
        .env
        .data_dir
        .join("service-builds/shop")
        .join(&dep.id)
        .exists());
}

#[tokio::test]
async fn failed_build_does_not_touch_containers() {
    let h = harness().await;
    fake_stack(&h.runner);
    set_config(
        &h,
        "source_type = 'git', git_url = 'https://github.com/acme/shop.git', \
         images = '[{\"image\":\"localhost:5000/app\"}]'",
    )
    .await;
    h.runner
        .on("git rev-parse HEAD", CommandOutput::ok("abcdef0123\n"));
    h.runner.on(
        "docker build",
        CommandOutput::failed(1, "ERROR: failed to solve"),
    );

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "failed");
    assert!(dep.error_message.unwrap().contains("failed to solve"));
    assert!(!h.runner.ran("docker push"));
    assert!(!h.runner.ran(" up -d"));
    assert!(!h.runner.ran(" pull"));
}

#[tokio::test]
async fn path_source_uses_revision_file_and_build_arg_tag() {
    let h = harness().await;
    fake_stack(&h.runner);
    let src = h.env.data_dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let rev = h.env.data_dir.join("REVISION");
    std::fs::write(&rev, "efb7da8\n").unwrap();
    set_config(
        &h,
        &format!(
            "source_type = 'path', source_path = '{}', revision_file = '{}', images = '{}'",
            src.display(),
            rev.display(),
            r#"[{"image":"localhost:5000/app","dockerfile":"infra/Dockerfile"},{"image":"localhost:5000/worker","dockerfile":"/opt/megasign/Dockerfile.worker","context":"/opt/megasign","build_args":{"BASE":"localhost:5000/app:${TAG}"}}]"#
        ),
    )
    .await;

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "succeeded", "{:?}", dep.error_message);
    assert_eq!(dep.image_tag.as_deref(), Some("efb7da8"));
    let cmds = h.runner.commands();
    let builds: Vec<&String> = cmds
        .iter()
        .filter(|c| c.starts_with("docker build"))
        .collect();
    assert_eq!(builds.len(), 2);
    assert!(builds[0].contains("-t localhost:5000/app:efb7da8"));
    assert!(builds[1].contains("--build-arg BASE=localhost:5000/app:efb7da8 /opt/megasign"));
    assert!(!h.runner.ran("git clone"));
    // The source directory on the server is never deleted.
    assert!(src.exists());
}

#[tokio::test]
async fn registry_login_uses_a_private_docker_config_and_masks_the_password() {
    let h = harness().await;
    fake_stack(&h.runner);
    let src = h.env.data_dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    set_config(
        &h,
        &format!(
            "source_type = 'path', source_path = '{}', registry_username = 'bot', \
             registry_password = 'hunter2-secret', images = '[{{\"image\":\"ghcr.io/acme/app\"}}]'",
            src.display()
        ),
    )
    .await;
    h.runner.on(
        "docker login",
        CommandOutput::failed(1, "denied for hunter2-secret"),
    );

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "failed");
    let login = h
        .runner
        .calls
        .lock()
        .iter()
        .find(|c| {
            c.display()
                .contains("docker login ghcr.io -u bot --password-stdin")
        })
        .cloned()
        .expect("login ran");
    assert_eq!(login.stdin.as_deref(), Some("hunter2-secret"));
    assert!(login.env.iter().any(|(k, _)| k == "DOCKER_CONFIG"));
    let lines = logs(&h, &dep).await.join("\n");
    assert!(!lines.contains("hunter2-secret"), "{}", lines);
    assert!(!dep
        .error_message
        .unwrap_or_default()
        .contains("hunter2-secret"));
}

#[tokio::test]
async fn push_webhook_matches_repo_and_branch() {
    let h = harness().await;
    set_config(
        &h,
        "source_type = 'git', git_url = 'https://github.com/Acme/Shop.git', branch = 'main', \
         auto_deploy = 1, images = '[{\"image\":\"a\"}]'",
    )
    .await;
    let urls = [
        "https://github.com/acme/shop.git",
        "git@github.com:acme/shop.git",
    ];
    assert_eq!(
        services_for_push(&h.env.db, &urls, "main").await,
        vec![h.service_id.clone()]
    );
    assert!(services_for_push(&h.env.db, &urls, "dev").await.is_empty());
    assert!(
        services_for_push(&h.env.db, &["https://github.com/acme/other"], "main")
            .await
            .is_empty()
    );
    set_config(&h, "auto_deploy = 0").await;
    assert!(services_for_push(&h.env.db, &urls, "main").await.is_empty());
}

const MIGRATE: &str =
    r#"[{"name":"migrate","service":"web","command":"bun run db:migrate","timeout_secs":60}]"#;

#[tokio::test]
async fn pre_deploy_hook_runs_after_pull_and_before_up() {
    let h = harness().await;
    fake_stack(&h.runner);
    set_config(
        &h,
        &format!(
            "pre_deploy_hooks = '{}', post_deploy_hooks = '{}'",
            MIGRATE, r#"[{"service":"web","command":"sh -c \"echo warm\""}]"#
        ),
    )
    .await;
    h.runner.on(
        " run --rm -T --no-deps",
        CommandOutput::ok("migrations applied\n"),
    );

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "succeeded", "{:?}", dep.error_message);
    let pull = h.runner.position(" pull").unwrap();
    let hook = h
        .runner
        .position("run --rm -T --no-deps --name rivetr-svc-shop-pre-hook-")
        .unwrap();
    let up = h.runner.position(" up -d").unwrap();
    let post = h.runner.position("rivetr-svc-shop-post-hook-").unwrap();
    assert!(pull < hook && hook < up && up < post);
    assert!(h.runner.commands()[hook].ends_with(" web bun run db:migrate"));
    assert!(h.runner.commands()[post].ends_with(" web sh -c echo warm"));
    let lines = logs(&h, &dep).await;
    assert!(
        lines.iter().any(|l| l == "migrations applied"),
        "{:?}",
        lines
    );
}

#[tokio::test]
async fn failed_pre_deploy_hook_leaves_running_containers_untouched() {
    let h = harness().await;
    fake_stack(&h.runner);
    let dir = h.env.data_dir.join("services/shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("docker-compose.yml"), "# running file\n").unwrap();
    set_config(
        &h,
        &format!(
            "source_type = 'git', git_url = 'https://github.com/acme/shop.git', \
             images = '[{{\"image\":\"localhost:5000/app\"}}]', pre_deploy_hooks = '{}'",
            MIGRATE
        ),
    )
    .await;
    h.runner
        .on("git rev-parse HEAD", CommandOutput::ok("abcdef0123\n"));
    h.runner.on(
        " run --rm -T --no-deps",
        CommandOutput::failed(1, "relation exists"),
    );

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "failed");
    let err = dep.error_message.unwrap();
    assert!(
        err.contains("pre-deploy hook 'migrate' failed (exit code 1)"),
        "{}",
        err
    );
    assert!(err.contains("not changed"));
    assert!(!h.runner.ran(" up -d"));
    assert_eq!(
        std::fs::read_to_string(dir.join("docker-compose.yml")).unwrap(),
        "# running file\n"
    );
    // latest points at the image the stack was running before the build.
    let tag = h
        .runner
        .position("docker tag sha256:old localhost:5000/app:latest")
        .expect("latest re-tagged");
    let repush = h.runner.commands()[tag..]
        .iter()
        .any(|c| c == "docker push localhost:5000/app:latest");
    assert!(repush);
}

#[tokio::test]
async fn failed_post_deploy_hook_is_reported_without_failing_the_deploy() {
    let h = harness().await;
    fake_stack(&h.runner);
    set_config(&h, &format!("post_deploy_hooks = '{}'", MIGRATE)).await;
    h.runner
        .on(" run --rm -T --no-deps", CommandOutput::failed(2, "boom"));

    let dep = deploy(&h, manual()).await;
    assert_eq!(dep.status, "succeeded");
    assert!(dep
        .error_message
        .unwrap()
        .contains("post-deploy hook 'migrate' failed"));
    assert!(h.runner.ran(" up -d"));
}
