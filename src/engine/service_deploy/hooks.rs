//! Pre- and post-deploy hooks: a command run in a one-off container of a
//! compose service (`docker compose run --rm --no-deps`), so it gets that
//! service's image, env_file, environment, volumes and networks. This is the
//! database migration step that used to be a hand-run `docker run`.

use anyhow::Result;
use std::path::Path;
use std::time::Duration;

use super::runner::{CommandRunner, CommandSpec, DeployLog};
use super::DbLog;
use crate::db::ServiceDeployHook;

const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Split a command line like a POSIX shell would (quotes and backslashes),
/// without expanding anything. `None` for unbalanced quotes.
pub fn split_command(input: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => cur.push(ch),
                        None => return None,
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.peek() {
                            Some('"') | Some('\\') | Some('$') | Some('`') => {
                                cur.push(chars.next().unwrap());
                            }
                            _ => cur.push('\\'),
                        },
                        Some(ch) => cur.push(ch),
                        None => return None,
                    }
                }
            }
            '\\' => {
                in_word = true;
                if let Some(ch) = chars.next() {
                    cur.push(ch);
                }
            }
            c if c.is_whitespace() => {
                if in_word {
                    args.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        args.push(cur);
    }
    Some(args)
}

/// Container name for a hook run, so a timed-out hook can be removed.
pub fn hook_container_name(project: &str, deployment_id: &str, phase: &str, n: usize) -> String {
    let short: String = deployment_id.chars().take(8).collect();
    format!("{}-{}-hook-{}-{}", project, phase, short, n)
}

/// `docker compose run` for one hook. The command is passed to the image's
/// entrypoint exactly like `docker run <image> <command>`; use `sh -c "..."`
/// for shell features.
pub fn hook_command(
    compose_dir: &Path,
    project: &str,
    hook: &ServiceDeployHook,
    container_name: &str,
) -> Result<CommandSpec> {
    let argv = split_command(&hook.command)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Hook command has unbalanced quotes"))?;
    let mut args: Vec<String> = vec![
        "compose".into(),
        "-p".into(),
        project.into(),
        "run".into(),
        "--rm".into(),
        "-T".into(),
        "--no-deps".into(),
        "--name".into(),
        container_name.into(),
        hook.service.trim().into(),
    ];
    args.extend(argv);
    let timeout = hook.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS);
    Ok(CommandSpec::args("docker", args)
        .cwd(compose_dir)
        .timeout(Duration::from_secs(timeout))
        .streamed())
}

/// Run hooks in order. Stops at the first failure and returns its message.
pub async fn run_hooks(
    runner: &dyn CommandRunner,
    compose_dir: &Path,
    project: &str,
    deployment_id: &str,
    phase: &str,
    hooks: &[ServiceDeployHook],
    log: &DbLog,
) -> Result<(), String> {
    for (i, hook) in hooks.iter().enumerate() {
        let label = hook
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| hook.command.clone());
        let name = hook_container_name(project, deployment_id, phase, i + 1);
        let spec = match hook_command(compose_dir, project, hook, &name) {
            Ok(s) => s,
            Err(e) => return Err(format!("{}-deploy hook '{}': {}", phase, label, e)),
        };
        log.info(&format!(
            "Running {}-deploy hook {}/{} in {}: {}",
            phase,
            i + 1,
            hooks.len(),
            hook.service,
            label
        ))
        .await;
        log.info(&format!("$ {}", spec.display())).await;
        let started = std::time::Instant::now();
        match runner.run(&spec, Some(log)).await {
            Ok(out) if out.success => {
                log.info(&format!(
                    "Hook '{}' finished in {:.1}s",
                    label,
                    started.elapsed().as_secs_f64()
                ))
                .await;
            }
            Ok(out) => {
                return Err(format!(
                    "{}-deploy hook '{}' failed ({})",
                    phase,
                    label,
                    match out.code {
                        Some(c) => format!("exit code {}", c),
                        None => "killed".to_string(),
                    }
                ));
            }
            Err(e) => {
                // Timed out or could not start: make sure the container is gone.
                let _ = runner
                    .run(&CommandSpec::new("docker", &["rm", "-f", &name]), None)
                    .await;
                return Err(format!("{}-deploy hook '{}': {:#}", phase, label, e));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_like_a_shell() {
        assert_eq!(
            split_command("bun run db:migrate").unwrap(),
            vec!["bun", "run", "db:migrate"]
        );
        assert_eq!(
            split_command(r#"sh -c "php artisan migrate --force && echo 'done'""#).unwrap(),
            vec!["sh", "-c", "php artisan migrate --force && echo 'done'"]
        );
        assert_eq!(
            split_command("a 'b c' d\\ e").unwrap(),
            vec!["a", "b c", "d e"]
        );
        assert_eq!(split_command("x ''").unwrap(), vec!["x", ""]);
        assert!(split_command("echo 'oops").is_none());
    }

    #[test]
    fn hook_runs_one_off_container_without_dependencies() {
        let hook = ServiceDeployHook {
            name: Some("migrate".into()),
            service: "api".into(),
            command: "bun run db:migrate".into(),
            timeout_secs: Some(120),
        };
        let spec = hook_command(Path::new("/d"), "rivetr-svc-mega-sign", &hook, "n1").unwrap();
        assert_eq!(
            spec.display(),
            "docker compose -p rivetr-svc-mega-sign run --rm -T --no-deps --name n1 api bun run db:migrate"
        );
        assert_eq!(spec.timeout, Some(Duration::from_secs(120)));
    }
}
