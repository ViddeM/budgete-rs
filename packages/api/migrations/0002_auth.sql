-- Auth: users, sessions, and per-user scoping of all data.

CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider    TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    email       TEXT,
    name        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, provider_id)
);

-- Server-side sessions: a random token maps to a user with an expiry.
CREATE TABLE sessions (
    token       UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- Scope all existing tables to a user.
-- Clear any existing data first so the NOT NULL constraint can be added cleanly.
DELETE FROM transaction_groups;
DELETE FROM transactions;
DELETE FROM categories;
DELETE FROM groups;

ALTER TABLE transactions ADD COLUMN user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE categories   ADD COLUMN user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE groups       ADD COLUMN user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE;

-- Drop the global unique constraints and replace with per-user ones.
DROP INDEX categories_unique_toplevel_name;
DROP INDEX categories_unique_subcat_name;

CREATE UNIQUE INDEX categories_unique_toplevel_name
    ON categories(user_id, name)
    WHERE parent_id IS NULL;

CREATE UNIQUE INDEX categories_unique_subcat_name
    ON categories(user_id, name, parent_id)
    WHERE parent_id IS NOT NULL;

-- Groups: names were globally unique; now unique per user.
ALTER TABLE groups DROP CONSTRAINT groups_name_key;
CREATE UNIQUE INDEX groups_unique_name_per_user ON groups(user_id, name);

CREATE INDEX idx_transactions_user ON transactions(user_id);
CREATE INDEX idx_categories_user   ON categories(user_id);
CREATE INDEX idx_groups_user       ON groups(user_id);

-- The global dedup_hash constraint would prevent two different users from
-- importing the same bank transaction.  Replace it with a per-user unique index.
ALTER TABLE transactions DROP CONSTRAINT transactions_dedup_hash_key;
CREATE UNIQUE INDEX transactions_unique_dedup_per_user ON transactions(user_id, dedup_hash);
