# KeyDungeon API — Project Overview

KeyDungeon is a small Rust backend for a password manager. It exposes a JSON
HTTP API that lets a user register, sign in, and manage an encrypted personal
vault of service credentials. Every account-scoped request is authenticated
with a JWT, and every vault secret is encrypted at rest.

This document is the high-level map of the project: how requests flow, what
each layer is responsible for, which endpoints exist, and how the security
model works.

## Stack

| Concern            | Choice                                                        |
| ------------------ | ------------------------------------------------------------- |
| Language           | Rust 2024 edition                                             |
| HTTP framework     | Axum 0.8                                                      |
| Async runtime      | Tokio                                                         |
| Database           | SQLite via SQLx 0.9                                           |
| Master password    | Argon2id via `argon2` + `password-hash`                       |
| Vault encryption   | ChaCha20-Poly1305 via `chacha20poly1305`                      |
| Authentication     | JWT bearer tokens via `jsonwebtoken`                          |
| Secret handling    | `secrecy` (`SecretString`) and `zeroize`                      |
| Observability      | `tracing` + `tracing-subscriber`, `tower-http` trace layer    |
| Middleware         | `tower-http` CORS, request body limit, request timeout, trace |

## Request flow

1. A request reaches the Axum router in `src/router.rs`.
2. Global layers apply: tracing, CORS, a 1 MiB body limit, and a 30 second
   timeout.
3. Public routes (`/health`, `/auth/login`, `/auth/register`) run directly.
4. Protected routes (`/auth/verify`, `/users/*`, `/passwords/*`, `/events`)
   pass through `jwt_middleware`. It extracts the `Authorization: Bearer`
   token, validates it with `TokenManager`, and inserts the decoded `Claims`
   into request extensions.
5. A handler reads `claims.sub` (the authenticated user UUID), calls one or
   more repositories, records an audit event, and returns JSON.
6. Repositories own all SQL. `PasswordRepository` additionally owns all vault
   encryption and decryption, so handlers never see ciphertext.

## Module map

```text
src/
  main.rs                       startup, tracing, SQLite pool, schema, AppState
  router.rs                     public + protected routers and global layers
  api/
    auth_resource.rs            register, login, verify
    user_resource.rs            read/update/delete the current user
    password_resources.rs       password CRUD (vault entries)
    event_resource.rs           read the current user's audit events
    health_resource.rs          liveness + database check
  handlers/
    claims.rs                   JWT claim shape and expiry helper
    jwt.rs                      TokenManager + JWT middleware
    events.rs                   fire-and-forget audit event helper
  helpers/
    password.rs                 Argon2id hashing/verification (+ pepper)
    vault.rs                    ChaCha20-Poly1305 encrypt/decrypt
  models/
    app_state.rs                shared state: pool, repositories, JWT manager
    auth.rs                     login/register request + response
    user.rs                     user row + update request
    password.rs                 vault entry row
    event.rs                    audit event row + event type enum
    error_response.rs           uniform `{ "error": "..." }` body
    health.rs                   health response
  repositories/
    user_repository.rs          signup and credential mutations
    password_repository.rs      encrypted vault reads/writes
    event_repository.rs         audit event reads/writes
init.sql                        SQLite schema (applied at startup)
```

## User handling

User CRUD is deliberately narrow. There is no public "list all users" or
"update any user" endpoint.

- **Create**: `POST /auth/register` creates the account. The handler validates
  input, calls `UserRepository::create`, and the repository hashes the master
  password with Argon2id before inserting the row.
- **Read**: `GET /users/{uuid}` returns the authenticated user's own profile.
  It refuses to return another user's record.
- **Update**: `PATCH /users/{uuid}` updates the current user's username and/or
  global master password. Password changes are hashed inside
  `UserRepository::update`, so raw passwords never reach SQL.
- **Delete**: `DELETE /users/{uuid}` deletes the current user. Vault entries
  cascade with the account.

`UserRepository` is the single owner of account creation and global-password
updates. Handlers therefore never hash a password themselves, and the Argon2
pepper is applied consistently.

### Confirming the caller is the right user

Authentication and authorization are separate steps:

1. `jwt_middleware` proves the token is valid and unexpired, producing
   `claims.sub`.
2. Every user handler calls `ensure_owner(&claims, uuid)` and returns `403`
   unless `claims.sub == uuid`.
3. Password and event queries are scoped by `claims.sub` in SQL
   (`WHERE user_uuid = ?`), so a valid token cannot read another vault.
4. `POST /auth/verify` re-checks that the account still exists, which lets a
   client detect tokens for deleted users before making other calls.

A missing or foreign password returns `404`, not `403`, to avoid confirming
that another user's entry exists.

## Password vault

`PasswordRepository` is the only component that encrypts or decrypts vault
data.

- **Create**: `POST /passwords/` encrypts the secret with
  ChaCha20-Poly1305 and stores the base64 payload.
- **List**: `GET /passwords/` loads a page of rows for the authenticated user
  and decrypts each secret before returning JSON. Pages are controlled with the
  `page` and `limit` query parameters (default `20`, max `100`), and the
  response includes the user's total entry count. An optional `search`
  parameter filters entries by a case-insensitive substring match on the name;
  `total` then reflects the filtered count.
- **Reveal**: `GET /passwords/{uuid}` decrypts one secret, scoped by both the
  entry UUID and the authenticated user UUID.
- **Update**: `PATCH /passwords/{uuid}` re-encrypts the submitted secret and
  only updates rows owned by the caller.
- **Delete**: `DELETE /passwords/{uuid}` deletes only rows owned by the caller
  and returns `204 No Content`.

Encryption format: a fresh random 96-bit nonce is generated for every write,
prepended to the ciphertext, and the combined value is base64-encoded into the
`passwords.password` column. The key comes from `VAULT_KEY`.

`Password` zeroizes its secret when dropped. Decrypted values are held in
`Zeroizing<String>` inside the vault helper.

## Audit events

Handlers emit an event for security-relevant actions through
`handlers::events::record_event`. Event writes are best-effort: a failed audit
insert logs a warning but never fails the user's request.

| Event type       | Emitted when                         |
| ---------------- | ------------------------------------ |
| `SignIn`         | successful login                     |
| `CreateUser`     | successful registration              |
| `ShowUser`       | reading the current user             |
| `EditUser`       | updating username or master password |
| `DeleteUser`     | deleting the current account         |
| `CreatePassword` | creating a vault entry               |
| `ShowPasswords`  | listing vault entries                |
| `RevealPassword` | reading one decrypted secret         |
| `EditPassword`   | updating a vault entry               |
| `DeletePassword` | deleting a vault entry               |

`GET /events` returns the current user's audit events newest-first, paginated
with the `page` and `limit` query parameters (default `20`, max `100`). The
`total` field in the response is the full event count for the user. The events
table intentionally has no foreign key to `users`, so the audit trail
(including the final `DeleteUser` event) survives account deletion.

## API

Public:

| Method | Path             | Purpose                                   |
| ------ | ---------------- | ----------------------------------------- |
| `GET`  | `/health`        | Process and database liveness             |
| `POST` | `/auth/register` | Create an account and return a JWT        |
| `POST` | `/auth/login`    | Authenticate and return a JWT             |

Requires `Authorization: Bearer <token>`:

| Method   | Path               | Purpose                                  |
| -------- | ------------------ | ---------------------------------------- |
| `POST`   | `/auth/verify`     | Validate token and confirm user exists   |
| `GET`    | `/users/{uuid}`    | Read the current user                    |
| `PATCH`  | `/users/{uuid}`    | Update username / master password        |
| `DELETE` | `/users/{uuid}`    | Delete the current user                  |
| `GET`    | `/passwords/`      | List vault entries, paginated (decrypted secrets) |
| `POST`   | `/passwords/`      | Create a vault entry                     |
| `GET`    | `/passwords/{uuid}`| Reveal one vault secret                  |
| `PATCH`  | `/passwords/{uuid}`| Update a vault entry                     |
| `DELETE` | `/passwords/{uuid}`| Delete a vault entry                     |
| `GET`    | `/events`          | List the current user's audit log, paginated |

Errors use a consistent body:

```json
{ "error": "Password not found" }
```

## Database

The schema lives in `init.sql` and is applied on every startup with
`CREATE TABLE IF NOT EXISTS`, so it is safe to rerun. UUIDs are stored as
SQLite `BLOB`s and timestamps as `TEXT`.

- `users(uuid, username, password, timestamp)` — `password` is an Argon2 PHC
  hash, never plaintext.
- `passwords(uuid, user_uuid, icon, url, name, username, password, timestamp)`
  — `password` is the encrypted base64 vault payload; `user_uuid` cascades on
  user deletion.
- `events(uuid, user_uuid, event_type, metadata, timestamp)` — append-only
  audit log, intentionally not cascaded.

SQLite runs with `foreign_keys=ON`, WAL journaling, `synchronous=NORMAL`, and
a 5 second busy timeout.

## Configuration

Copy `.env.example` to `.env` and set the values:

| Variable           | Required | Purpose                                                      |
| ------------------ | -------- | ------------------------------------------------------------ |
| `DATABASE_URL`     | No       | SQLite URL, defaults to `sqlite://keydungeon.db`             |
| `BACKUP_DIR`       | No       | Backup output directory (default `./backups`)                |
| `BACKUP_MAX_FILES` | No       | Number of backups to keep (default `3`)                      |
| `VAULT_KEY`        | Yes      | Base64 32-byte ChaCha20-Poly1305 key                         |
| `PASSWORD_PEPPER`  | No*      | Server-side Argon2 pepper                                    |
| `JWT_SECRET`       | No*      | HMAC secret for JWT signing                                  |
| `JWT_VALID_TIME`   | No       | Token lifetime in hours (default `24`)                       |
| `RUST_LOG`         | No       | Tracing filter (default `info`)                              |
| `PORT`             | No       | Listen port (default `4321`)                                 |

\* A development fallback exists, but the process logs a warning. Set strong
random values in any real deployment.

Generate secrets with:

```sh
openssl rand -base64 32   # VAULT_KEY, PASSWORD_PEPPER, JWT_SECRET
```

## Running

Local:

```sh
cp .env.example .env
cargo run
```

Container:

```sh
docker compose up --build
```

The container bind-mounts the host's `~/keydungeon/data` directory at `/data`
and `~/keydungeon/backups` at `/backups`, and overrides `DATABASE_URL` to
`sqlite:///data/keydungeon.db`, so the database is persisted on disk at
`~/keydungeon/data/keydungeon.db`.

### Backups

The API runs an in-process scheduler (`tokio-cron-scheduler`) that creates a
consistent SQLite snapshot every 6 hours using `VACUUM INTO`. Backups are
written to `BACKUP_DIR` as `keydungeon-YYYYMMDD-HHMMSS.db`, first to a `.tmp`
file and then renamed into place so partial files are never left behind. After
each successful backup the oldest files are pruned, keeping the newest
`BACKUP_MAX_FILES` (default 3). Scheduler startup failures are logged but do not
prevent the API from starting.

On startup the API also runs a catch-up snapshot when no backup exists or the
newest one is older than 6 hours. This backs up a fresh deployment immediately
while avoiding retention churn from frequent restarts. Set `BACKUP_ON_START` to
`false` to disable it.

`VACUUM INTO` produces a complete single-file copy, so it is safe to run while
the API is serving requests. Back up `~/keydungeon/backups` to external storage
instead of copying the live WAL-mode database files.

## Verification

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
```

Unit tests cover Argon2 hashing/verification (including pepper separation) and
vault encryption/decryption (including tamper detection and key-length
validation).

## Operational notes

- Back up `VAULT_KEY`. Without it, stored vault secrets cannot be recovered.
- Back up the SQLite database file (and its WAL files) consistently.
- Rotating `PASSWORD_PEPPER` invalidates every existing master-password hash.
- Rotating `VAULT_KEY` requires re-encrypting every vault entry.
- The default JWT secret and pepper exist only for local development.
