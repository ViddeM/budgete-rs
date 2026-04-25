CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE categories (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL UNIQUE,
    color       TEXT NOT NULL DEFAULT '#6366f1',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE groups (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE transactions (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    date        DATE,
    description TEXT NOT NULL,
    amount      NUMERIC(12, 2) NOT NULL,
    source      TEXT NOT NULL,
    currency    TEXT NOT NULL DEFAULT 'SEK',
    dedup_hash  TEXT NOT NULL UNIQUE,
    is_pending  BOOLEAN NOT NULL DEFAULT FALSE,
    category_id UUID REFERENCES categories(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE transaction_groups (
    transaction_id UUID NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    group_id       UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (transaction_id, group_id)
);

CREATE INDEX idx_transactions_category ON transactions(category_id);
CREATE INDEX idx_transactions_date ON transactions(date);
CREATE INDEX idx_transactions_is_pending ON transactions(is_pending);
CREATE INDEX idx_transaction_groups_group ON transaction_groups(group_id);
