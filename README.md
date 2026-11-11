# Keydungeon API

A Rust backend for an encrypted password manager, built with:

- [Axum](https://github.com/tokio-rs/axum) 0.8
- [Tokio](https://tokio.rs/)
- [SQLx](https://github.com/launchbadge/sqlx) + SQLite
- [Argon2id](https://docs.rs/argon2) for master-password hashing
- [ChaCha20-Poly1305](https://docs.rs/chacha20poly1305) for vault encryption
- [jsonwebtoken](https://docs.rs/jsonwebtoken) for JWT authentication
- [Secrecy](https://docs.rs/secrecy) and [Zeroize](https://docs.rs/zeroize) for secret handling

See [PROJECT.md](PROJECT.md) for the full architecture overview, security
model, API endpoint list, and configuration reference.

## Running with Docker

```sh
cp .env.example .env   # then fill in the secrets
docker compose up -d --build
```

The SQLite database lives on the `keydungeon_data` named volume mounted at
`/data`, so restarting or rebuilding the container keeps all data. `init.sql`
is applied on every start and only uses `CREATE ... IF NOT EXISTS`, so it never
truncates or overwrites existing tables. Removing the volume
(`docker compose down -v`) does delete the database.

The runtime image runs as the non-root user `65532`. A volume created by an
older root-running image is owned by root, so grant access once before the
first start of the new image:

```sh
docker run --rm -v keydungeon-api_keydungeon_data:/data \
  alpine chown -R 65532:65532 /data
```

Open API V3.0.3 Spec:

`open-api-spec-keydungeon.yaml`
