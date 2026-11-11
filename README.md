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

The SQLite database is bind-mounted from `~/keydungeon/data` on the host to
`/data` in the container, so the database file is
`~/keydungeon/data/keydungeon.db`. Restarting or rebuilding the container keeps
the data, and `docker compose down` does not delete it; deleting the directory
does. `init.sql` is applied on every start and only uses
`CREATE ... IF NOT EXISTS`, so it never truncates or overwrites existing
tables.

While running, the API writes a consistent SQLite snapshot every 6 hours to
`~/keydungeon/backups` (mounted at `/backups`). It keeps the newest 3 files and
prunes older ones. On startup it also takes a catch-up snapshot when no backup
exists or the newest one is older than 6 hours, so a fresh deploy is backed up
immediately without frequent restarts pushing older backups out of the 3-file
retention. Set `BACKUP_ON_START=false` to disable the startup snapshot.

The backups use SQLite's `VACUUM INTO`, so each file is a complete, standalone
database that is safe to copy even while the API is serving requests. Point
your sync tool at `~/keydungeon/backups`, **not** at the live `data` directory,
because the live database is in WAL mode and copying its `.db`/`-wal`/`-shm`
files while they are being written can produce an inconsistent snapshot.

The runtime image runs as the non-root user `65532`. On Linux, make sure the
host directories are writable by that user before the first start:

```sh
mkdir -p ~/keydungeon/data ~/keydungeon/backups
sudo chown -R 65532:65532 ~/keydungeon/data ~/keydungeon/backups
```

On Docker Desktop (Windows/macOS), Docker manages bind-mount permissions.

If you are switching from the old `keydungeon_data` named volume (or an
existing `./data` directory), copy the existing database into
`~/keydungeon/data` before starting the new container:

```sh
mkdir -p ~/keydungeon/data
docker compose stop
docker cp keydungeon-api:/data/. ~/keydungeon/data/
docker compose up -d
```

After verifying the data, the old volume can be removed with
`docker volume rm keydungeon-api_keydungeon_data`.

### Restoring a backup

Stop the API, replace the live database with a backup, then start it again:

```sh
docker compose stop
cp ~/keydungeon/data/keydungeon.db ~/keydungeon/data/keydungeon.db.before-restore
cp ~/keydungeon/backups/keydungeon-YYYYMMDD-HHMMSS.db ~/keydungeon/data/keydungeon.db
rm -f ~/keydungeon/data/keydungeon.db-wal ~/keydungeon/data/keydungeon.db-shm
docker compose start
```

Always test a restore before relying on the backups.

Open API V3.0.3 Spec:

`open-api-spec-keydungeon.yaml`
