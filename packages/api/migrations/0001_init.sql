CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Two-level category hierarchy.
-- Top-level categories have parent_id = NULL.
-- Subcategories have parent_id pointing to a top-level category.
-- Deleting a parent cascades to its subcategories; the ON DELETE SET NULL
-- on transactions.category_id then unclassifies any transactions that were
-- assigned to a deleted subcategory.
CREATE TABLE categories (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL,
    color       TEXT NOT NULL DEFAULT '#6366f1',
    parent_id   UUID REFERENCES categories(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Top-level names are unique among themselves.
CREATE UNIQUE INDEX categories_unique_toplevel_name
    ON categories(name)
    WHERE parent_id IS NULL;

-- Subcategory names are unique within their parent.
CREATE UNIQUE INDEX categories_unique_subcat_name
    ON categories(name, parent_id)
    WHERE parent_id IS NOT NULL;

CREATE INDEX idx_categories_parent ON categories(parent_id);

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
