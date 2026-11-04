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
