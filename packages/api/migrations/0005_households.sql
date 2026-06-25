-- Households: a shared unit that scopes all financial data.

CREATE TABLE households (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL,
    invite_code TEXT NOT NULL UNIQUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add household membership to users (nullable until setup completes).
ALTER TABLE users ADD COLUMN household_id UUID REFERENCES households(id);

-- Backfill: create one household per existing user.
CREATE TEMP TABLE _user_household_map AS
SELECT
    u.id AS user_id,
    uuid_generate_v4() AS household_id,
    COALESCE(u.name, u.email, 'My Household') AS household_name,
    UPPER(SUBSTR(REPLACE(uuid_generate_v4()::text, '-', ''), 1, 4)) || '-' ||
    UPPER(SUBSTR(REPLACE(uuid_generate_v4()::text, '-', ''), 1, 4)) AS invite_code
FROM users u;

INSERT INTO households (id, name, invite_code)
SELECT household_id, household_name, invite_code FROM _user_household_map;

UPDATE users u
SET household_id = m.household_id
FROM _user_household_map m
WHERE u.id = m.user_id;

DROP TABLE _user_household_map;

-- Add household_id to data tables, populate from user's household, then drop user_id.
ALTER TABLE transactions ADD COLUMN household_id UUID;
ALTER TABLE categories   ADD COLUMN household_id UUID;
ALTER TABLE groups       ADD COLUMN household_id UUID;

UPDATE transactions t SET household_id = u.household_id FROM users u WHERE t.user_id = u.id;
UPDATE categories   c SET household_id = u.household_id FROM users u WHERE c.user_id = u.id;
UPDATE groups       g SET household_id = u.household_id FROM users u WHERE g.user_id = u.id;

ALTER TABLE transactions ALTER COLUMN household_id SET NOT NULL;
ALTER TABLE categories   ALTER COLUMN household_id SET NOT NULL;
ALTER TABLE groups       ALTER COLUMN household_id SET NOT NULL;

ALTER TABLE transactions ADD CONSTRAINT transactions_household_fk
    FOREIGN KEY (household_id) REFERENCES households(id) ON DELETE CASCADE;
ALTER TABLE categories ADD CONSTRAINT categories_household_fk
    FOREIGN KEY (household_id) REFERENCES households(id) ON DELETE CASCADE;
ALTER TABLE groups ADD CONSTRAINT groups_household_fk
    FOREIGN KEY (household_id) REFERENCES households(id) ON DELETE CASCADE;

DROP INDEX idx_transactions_user;
DROP INDEX idx_categories_user;
DROP INDEX idx_groups_user;
DROP INDEX transactions_unique_dedup_per_user;
DROP INDEX categories_unique_toplevel_name;
DROP INDEX categories_unique_subcat_name;
DROP INDEX groups_unique_name_per_user;

ALTER TABLE transactions DROP COLUMN user_id;
ALTER TABLE categories   DROP COLUMN user_id;
ALTER TABLE groups       DROP COLUMN user_id;

CREATE UNIQUE INDEX transactions_unique_dedup_per_household
    ON transactions(household_id, dedup_hash);
CREATE UNIQUE INDEX categories_unique_toplevel_name
    ON categories(household_id, name)
    WHERE parent_id IS NULL;
CREATE UNIQUE INDEX categories_unique_subcat_name
    ON categories(household_id, name, parent_id)
    WHERE parent_id IS NOT NULL;
CREATE UNIQUE INDEX groups_unique_name_per_household
    ON groups(household_id, name);

CREATE INDEX idx_transactions_household ON transactions(household_id);
CREATE INDEX idx_categories_household   ON categories(household_id);
CREATE INDEX idx_groups_household       ON groups(household_id);
