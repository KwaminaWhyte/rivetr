//! Additional developer tools service templates

use super::TemplateEntry;

pub fn templates() -> Vec<TemplateEntry> {
    vec![
        // ==================== DEVELOPER TOOLS (additional) ====================
        (
            "tpl-gitlab-ce",
            "GitLab CE",
            "Complete DevOps platform in a single container. Git hosting, CI/CD, registry, and more.",
            "development",
            "gitlab",
            r#"services:
  gitlab:
    image: gitlab/gitlab-ce:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitlab}
    restart: unless-stopped
    hostname: ${GITLAB_HOSTNAME:-gitlab.example.com}
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${SSH_PORT:-2222}:22"
    environment:
      - GITLAB_OMNIBUS_CONFIG=external_url 'http://${GITLAB_HOSTNAME:-gitlab.example.com}:${HTTP_PORT:-80}'
    volumes:
      - gitlab_config:/etc/gitlab
      - gitlab_logs:/var/log/gitlab
      - gitlab_data:/var/opt/gitlab
    shm_size: 256m
    labels:
      - "rivetr.managed=true"

volumes:
  gitlab_config:
  gitlab_logs:
  gitlab_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitlab","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"2222","secret":false},{"name":"GITLAB_HOSTNAME","label":"GitLab Hostname","required":false,"default":"gitlab.example.com","secret":false}]"#,
        ),
        (
            "tpl-woodpecker-ci",
            "Woodpecker CI",
            "Community fork of Drone CI. Container-native CI/CD with YAML pipeline configuration.",
            "development",
            "woodpecker",
            r#"services:
  woodpecker-server:
    image: woodpeckerci/woodpecker-server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-woodpecker-server}
    restart: unless-stopped
    ports:
      - "${PORT:-8000}:8000"
      - "${GRPC_PORT:-9000}:9000"
    environment:
      - WOODPECKER_OPEN=${OPEN_REGISTRATION:-false}
      - WOODPECKER_HOST=${SERVER_HOST:-http://localhost:8000}
      - WOODPECKER_GITHUB=${ENABLE_GITHUB:-true}
      - WOODPECKER_GITHUB_CLIENT=${GITHUB_CLIENT:-}
      - WOODPECKER_GITHUB_SECRET=${GITHUB_SECRET:-}
      - WOODPECKER_AGENT_SECRET=${AGENT_SECRET:-change-me-to-a-random-string}
      - WOODPECKER_DATABASE_DRIVER=sqlite3
      - WOODPECKER_DATABASE_DATASOURCE=/var/lib/woodpecker/woodpecker.sqlite
    volumes:
      - woodpecker_server_data:/var/lib/woodpecker
    labels:
      - "rivetr.managed=true"

volumes:
  woodpecker_server_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"woodpecker-server","secret":false},{"name":"PORT","label":"Web UI Port","required":false,"default":"8000","secret":false},{"name":"GRPC_PORT","label":"gRPC Port","required":false,"default":"9000","secret":false},{"name":"SERVER_HOST","label":"Server Host URL","required":true,"default":"http://localhost:8000","secret":false},{"name":"AGENT_SECRET","label":"Agent Secret","required":true,"default":"","secret":true},{"name":"GITHUB_CLIENT","label":"GitHub Client ID","required":false,"default":"","secret":false},{"name":"GITHUB_SECRET","label":"GitHub Client Secret","required":false,"default":"","secret":true},{"name":"OPEN_REGISTRATION","label":"Open Registration","required":false,"default":"false","secret":false}]"#,
        ),
        (
            "tpl-gitness",
            "Gitness",
            "Open-source code hosting and CI/CD platform by Harness. Git, pipelines, and artifact registry.",
            "development",
            "gitness",
            r#"services:
  gitness:
    image: harness/gitness:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitness}
    restart: unless-stopped
    ports:
      - "${PORT:-3000}:3000"
    environment:
      - GITNESS_HTTP_PORT=3000
      - GITNESS_PRINCIPAL_ADMIN_PASSWORD=${ADMIN_PASSWORD:-changeme}
      - GITNESS_URL_BASE=${BASE_URL:-http://localhost:3000}
    volumes:
      - gitness_data:/data
    labels:
      - "rivetr.managed=true"

volumes:
  gitness_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitness","secret":false},{"name":"PORT","label":"Port","required":false,"default":"3000","secret":false},{"name":"ADMIN_PASSWORD","label":"Admin Password","required":true,"default":"","secret":true},{"name":"BASE_URL","label":"Base URL","required":false,"default":"http://localhost:3000","secret":false}]"#,
        ),
        (
            "tpl-onedev",
            "OneDev",
            "All-in-one DevOps platform with Git, CI/CD, issue tracking, and code search.",
            "development",
            "onedev",
            r#"services:
  onedev:
    image: 1dev/server:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-onedev}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-6610}:6610"
      - "${SSH_PORT:-6611}:6611"
    volumes:
      - onedev_data:/opt/onedev
    labels:
      - "rivetr.managed=true"

volumes:
  onedev_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"onedev","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"6610","secret":false},{"name":"SSH_PORT","label":"SSH Port","required":false,"default":"6611","secret":false}]"#,
        ),
        (
            "tpl-docker-registry",
            "Docker Registry",
            "Private Docker image registry. Store and distribute container images within your infrastructure.",
            "development",
            "docker-registry",
            r#"services:
  registry:
    image: registry:${VERSION:-2}
    container_name: ${CONTAINER_NAME:-registry}
    restart: unless-stopped
    ports:
      - "${PORT:-5000}:5000"
    environment:
      - REGISTRY_STORAGE_DELETE_ENABLED=true
      - REGISTRY_HTTP_ADDR=0.0.0.0:5000
    volumes:
      - registry_data:/var/lib/registry
    labels:
      - "rivetr.managed=true"

volumes:
  registry_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"2","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"registry","secret":false},{"name":"PORT","label":"Port","required":false,"default":"5000","secret":false}]"#,
        ),
        (
            "tpl-verdaccio",
            "Verdaccio",
            "Lightweight private npm proxy registry. Cache packages and publish private packages.",
            "development",
            "verdaccio",
            r#"services:
  verdaccio:
    image: verdaccio/verdaccio:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-verdaccio}
    restart: unless-stopped
    ports:
      - "${PORT:-4873}:4873"
    environment:
      - VERDACCIO_PORT=4873
    volumes:
      - verdaccio_storage:/verdaccio/storage
      - verdaccio_conf:/verdaccio/conf
      - verdaccio_plugins:/verdaccio/plugins
    labels:
      - "rivetr.managed=true"

volumes:
  verdaccio_storage:
  verdaccio_conf:
  verdaccio_plugins:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"verdaccio","secret":false},{"name":"PORT","label":"Port","required":false,"default":"4873","secret":false}]"#,
        ),
        (
            "tpl-gitea-runner",
            "Gitea Actions Runner",
            "Gitea act_runner for running CI/CD workflows triggered by Gitea Actions.",
            "development",
            "gitea",
            r#"services:
  gitea-runner:
    image: gitea/act_runner:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-gitea-runner}
    restart: unless-stopped
    environment:
      - GITEA_INSTANCE_URL=${GITEA_URL:-http://gitea:3000}
      - GITEA_RUNNER_REGISTRATION_TOKEN=${REGISTRATION_TOKEN:-change-me}
      - GITEA_RUNNER_NAME=${RUNNER_NAME:-my-runner}
      - GITEA_RUNNER_LABELS=${RUNNER_LABELS:-ubuntu-latest:docker://node:18-bullseye}
      - CONFIG_FILE=/data/config.yaml
    volumes:
      - gitea_runner_data:/data
      - /var/run/docker.sock:/var/run/docker.sock
    labels:
      - "rivetr.managed=true"

volumes:
  gitea_runner_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"gitea-runner","secret":false},{"name":"GITEA_URL","label":"Gitea Instance URL","required":true,"default":"http://gitea:3000","secret":false},{"name":"REGISTRATION_TOKEN","label":"Runner Registration Token","required":true,"default":"","secret":true},{"name":"RUNNER_NAME","label":"Runner Name","required":false,"default":"my-runner","secret":false},{"name":"RUNNER_LABELS","label":"Runner Labels","required":false,"default":"ubuntu-latest:docker://node:18-bullseye","secret":false}]"#,
        ),
        (
            "tpl-nginx-proxy-manager",
            "Nginx Proxy Manager",
            "Easy-to-use reverse proxy with SSL termination and a beautiful web UI. No nginx config needed.",
            "infrastructure",
            "nginx",
            r#"services:
  nginx-proxy-manager:
    image: jc21/nginx-proxy-manager:${VERSION:-latest}
    container_name: ${CONTAINER_NAME:-nginx-proxy-manager}
    restart: unless-stopped
    ports:
      - "${HTTP_PORT:-80}:80"
      - "${HTTPS_PORT:-443}:443"
      - "${ADMIN_PORT:-81}:81"
    environment:
      - DB_MYSQL_HOST=npm_db
      - DB_MYSQL_PORT=3306
      - DB_MYSQL_USER=npm
      - DB_MYSQL_PASSWORD=${DB_PASSWORD:-npm}
      - DB_MYSQL_NAME=npm
    depends_on:
      - npm_db
    volumes:
      - npm_data:/data
      - npm_letsencrypt:/etc/letsencrypt
    labels:
      - "rivetr.managed=true"

  npm_db:
    image: jc21/mariadb-aria:${DB_VERSION:-latest}
    restart: unless-stopped
    environment:
      - MYSQL_ROOT_PASSWORD=${DB_ROOT_PASSWORD:-npm}
      - MYSQL_DATABASE=npm
      - MYSQL_USER=npm
      - MYSQL_PASSWORD=${DB_PASSWORD:-npm}
    volumes:
      - npm_db_data:/var/lib/mysql
    labels:
      - "rivetr.managed=true"

volumes:
  npm_data:
  npm_letsencrypt:
  npm_db_data:
"#,
            r#"[{"name":"VERSION","label":"Version","required":false,"default":"latest","secret":false},{"name":"CONTAINER_NAME","label":"Container Name","required":false,"default":"nginx-proxy-manager","secret":false},{"name":"HTTP_PORT","label":"HTTP Port","required":false,"default":"80","secret":false},{"name":"HTTPS_PORT","label":"HTTPS Port","required":false,"default":"443","secret":false},{"name":"ADMIN_PORT","label":"Admin UI Port","required":false,"default":"81","secret":false},{"name":"DB_PASSWORD","label":"Database Password","required":true,"default":"","secret":true},{"name":"DB_ROOT_PASSWORD","label":"DB Root Password","required":true,"default":"","secret":true}]"#,
        ),
    ]
}
