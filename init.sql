-- KeyDungeon SQLite schema.
--
-- UUIDs are stored as BLOBs (sqlx's SQLite UUID encoding) and timestamps are
-- stored as TEXT in RFC 3339 / `%F %T%.f` format. The schema is applied on
-- every startup with `CREATE ... IF NOT EXISTS`, so it is safe to rerun.

CREATE TABLE IF NOT EXISTS users (
    uuid BLOB PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Encrypted vault entries. `password` holds a base64 value produced by
-- ChaCha20-Poly1305 (nonce prepended to the ciphertext).
CREATE TABLE IF NOT EXISTS passwords (
    uuid BLOB PRIMARY KEY NOT NULL,
    user_uuid BLOB NOT NULL REFERENCES users(uuid) ON DELETE CASCADE,
    icon TEXT NOT NULL,
    url TEXT NOT NULL,
    name TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_passwords_user_uuid ON passwords(user_uuid);
CREATE INDEX IF NOT EXISTS idx_passwords_name ON passwords(name);

-- Audit log for user actions. `user_uuid` intentionally has no foreign key so
-- the audit trail (including the final `DeleteUser` event) survives account
-- deletion.
CREATE TABLE IF NOT EXISTS events (
    uuid BLOB PRIMARY KEY NOT NULL,
    user_uuid BLOB NOT NULL,
    event_type TEXT NOT NULL,
    metadata TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_user_uuid ON events(user_uuid);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
