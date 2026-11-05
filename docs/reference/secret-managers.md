# External secret managers

Rivetr can read environment variable values from HashiCorp Vault, OpenBao,
Infisical, Doppler and AWS Secrets Manager when it deploys. The variable stores
a reference; the value is fetched at deploy time, kept in memory for that run,
masked in the deployment log and never written to the Rivetr database.

Connections live under **Integrations > Secret managers** (`/integrations/secrets`).

## Reference syntax

```
secret://<connection>/<path>#<key>
```

| Part | Meaning |
|---|---|
| `<connection>` | Connection name: lowercase letters, digits, `-`, `_`. Looked up in the resource's team first, then instance-wide connections |
| `<path>` | Provider path, see the table below. May be empty |
| `#<key>` | One key of the secret. Required except for AWS Secrets Manager, where leaving it out uses the whole `SecretString` |

| Provider | Path | Example |
|---|---|---|
| Vault, OpenBao | KV v2 secret path inside the connection's mount | `secret://vault-prod/myapp/config#DATABASE_URL` |
| Infisical | Folder (empty for `/`) in the connection's project and environment | `secret://infisical/backend#STRIPE_KEY` |
| Doppler | Empty (the token's config) or `project/config` | `secret://doppler/#API_KEY` |
| AWS Secrets Manager | Secret name or ARN | `secret://aws/prod/db#password`, `secret://aws/prod/api-token` |

References work in:

- app variables, environment variables, project and team shared variables (all
  layers are resolved for the app at deploy, rollback, zero-downtime restart and
  preview deploys)
- compose service variables (written to the service's `.env` on deploy, start
  and restart)
- app build secrets (BuildKit `--secret`)

Not resolved: build-time env passed to Nixpacks, Railpack, buildpacks and static
builds (those get the stored value), replica scale-up from the Replicas page, and
managed databases (they have no custom env vars).

### Importing a whole path

The **Secret manager** card on an app's, service's or project's environment page
links every key at a path. Each key becomes a variable named
`<prefix><KEY>`; keys that are not valid variable names are skipped with a
warning. Variables you define yourself always win over imported keys, and app
imports win over project imports. Project imports apply to every app in the
project (not to compose services, which do not inherit project variables).

### Compose services

A reference in a service variable stays `${KEY}` in the rendered
`docker-compose.yml`; compose fills it from the Rivetr-managed `.env` (mode
0600). Use `${KEY}` rather than `${KEY:?message}` for imported keys: required
variable checks only see variables defined on the service.

## When a provider fails

Per app or service (Secret manager card):

- **Fail the deploy** (default): the deploy, restart or service start stops
  before any running container is touched, with an error that names the
  variable and the reason (never the value).
- **Use the last resolved values**: after each successful resolution the values
  are stored encrypted (instance encryption key). If a provider is unreachable
  later, those values are used and the deployment log shows a warning with the
  time they were resolved. A reference added since the last success still fails.
  Switching back to "fail" deletes the stored values. Without an encryption key
  nothing is stored and this policy cannot fall back.

## Security

- Credentials are write-only: they are encrypted with the instance encryption
  key (an encryption key is required to add a connection) and the API only says
  which credential fields are set.
- Team admins and owners manage their team's connections; instance-wide
  connections (no team) need an instance administrator. Any team member can
  test a connection and browse it.
- Browsing returns folder names and key names only. Secret values never reach
  the browser.
- For non-admins, connection URLs go through the outbound policy
  (Settings > Advanced > Outbound): a Vault on a private address needs its CIDR
  allowed there.
- Resolved values are masked as `********` in deployment logs, log drains and
  stored deployment errors (values of 4+ characters, plus each line of 8+
  characters of a multi-line value).
- Each deploy reads a path once; the cache is dropped when the deployment ends.
- Create, update and delete of connections, imports and policies are in the
  audit log.

## Provider setup

### HashiCorp Vault and OpenBao

Fields: address, KV v2 mount (default `secret`), namespace (optional), auth.

- **Token**: a token with a policy like
  ```hcl
  path "secret/data/myapp/*"     { capabilities = ["read"] }
  path "secret/metadata/myapp/*" { capabilities = ["list"] }
  ```
  `list` on `metadata` is only needed to browse. Test calls
  `auth/token/lookup-self`.
- **AppRole**: role ID (config) and secret ID (credential); the AppRole mount
  defaults to `approle`. Rivetr logs in once per deploy.

```bash
vault auth enable approle
vault write auth/approle/role/rivetr token_policies=rivetr token_ttl=15m
vault read auth/approle/role/rivetr/role-id
vault write -f auth/approle/role/rivetr/secret-id
```

The namespace is sent as `X-Vault-Namespace`. Only KV version 2 is supported.

### Infisical

Create a machine identity with **Universal Auth**, give it read access to the
project, and enter: site URL (default `https://app.infisical.com`, change it for
self-hosted or EU), project ID, environment slug (`dev`, `staging`, `prod`),
client ID and client secret. The path is the folder; secret references and
imports inside Infisical are expanded.

### Doppler

Create a **service token** for the config (`dp.st.<config>.<...>`). Project and
config are optional because the token is already scoped; set them when using a
personal or service account token, or override per reference with
`secret://doppler/<project>/<config>#KEY`.

### AWS Secrets Manager

Fields: region, auth, optional endpoint override (VPC endpoint).

- **Access key**: an IAM user or role key pair (plus session token for
  temporary credentials).
- **EC2 instance role**: credentials come from the instance metadata service
  (IMDSv2). Only works when Rivetr runs on EC2 with an instance profile.

Minimal policy:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    { "Effect": "Allow", "Action": "secretsmanager:GetSecretValue",
      "Resource": "arn:aws:secretsmanager:eu-west-1:123456789012:secret:prod/*" },
    { "Effect": "Allow", "Action": "secretsmanager:ListSecrets", "Resource": "*" }
  ]
}
```

`ListSecrets` is only needed to browse; test reports success without it. JSON
secrets are split into keys (`#password`); plain-text secrets are used whole
(no `#key`). Binary secrets are not supported. Requests are signed with SigV4
by Rivetr itself (no AWS SDK).

## API

All under `/api`, bearer token auth.

| Method | Path | Notes |
|---|---|---|
| GET | `/secret-connections?team_id=` | Team's connections plus instance-wide ones |
| POST | `/secret-connections` | `{team_id, name, provider, config, credentials}`; provider is `vault`, `openbao`, `infisical`, `doppler` or `aws_secrets_manager` |
| GET, PUT, DELETE | `/secret-connections/{id}` | PUT: blank credential fields keep the stored value |
| POST | `/secret-connections/{id}/test` | `{ok, message}`, recorded on the connection |
| POST | `/secret-connections/test` | Test unsaved settings `{provider, config, credentials}` |
| GET | `/secret-connections/{id}/browse?path=` | `{entries: [{name, path, kind}], keys: [...]}`, no values |
| GET, POST | `/{apps,services,projects}/{id}/secret-imports` | POST `{connection_id, path, prefix}` |
| DELETE | `/{apps,services,projects}/{id}/secret-imports/{import_id}` | |
| GET, PUT | `/{apps,services}/{id}/secret-policy` | `{failure_policy: "fail" \| "last_known"}` |

Config fields: `auth_method`, `address`, `mount`, `namespace`, `role_id`,
`approle_mount`, `site_url`, `project_id`, `environment`, `client_id`,
`api_url`, `project`, `config`, `region`, `access_key_id`, `endpoint`.
Credential fields: `token`, `secret_id`, `client_secret`, `secret_access_key`,
`session_token`.

## Storage (migration 135)

- `secret_connections`: connection rows, `credentials` encrypted JSON, unique
  name per team.
- `secret_imports`: owner (`app`, `service`, `project`), connection, path, prefix.
- `secret_resolution_state`: failure policy per owner and, for `last_known`
  only, the last resolved values (encrypted).
