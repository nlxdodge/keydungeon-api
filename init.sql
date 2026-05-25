-- Create users table
CREATE TABLE IF NOT EXISTS users (
    uuid UUID PRIMARY KEY,
    username VARCHAR(255) NOT NULL UNIQUE,
    password VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on username for faster lookups
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- Passwords table (stores encrypted passwords for different services)
CREATE TABLE IF NOT EXISTS passwords (
    uuid UUID PRIMARY KEY,
    user_uuid UUID NOT NULL REFERENCES users(uuid) ON DELETE CASCADE,
    icon VARCHAR(511),
    url VARCHAR(511),
    name VARCHAR(255) NOT NULL,
    username VARCHAR(255),
    password VARCHAR(255) NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for passwords table
CREATE INDEX IF NOT EXISTS idx_passwords_user_uuid ON passwords(user_uuid);
CREATE INDEX IF NOT EXISTS idx_passwords_name ON passwords(name);

-- Events table (audit log for user actions)
CREATE TABLE IF NOT EXISTS events (
    uuid UUID PRIMARY KEY,
    user_uuid UUID NOT NULL REFERENCES users(uuid) ON DELETE CASCADE,
    event_type VARCHAR(255) NOT NULL,
    metadata VARCHAR(1023),
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for events table
CREATE INDEX IF NOT EXISTS idx_events_user_uuid ON events(user_uuid);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
