-- Add two-level category hierarchy.
-- Top-level categories have parent_id = NULL.
-- Subcategories have parent_id pointing to a top-level category.
-- Deleting a parent cascades to its subcategories; the existing
-- ON DELETE SET NULL on transactions.category_id then unclassifies
-- any transactions that were assigned to a deleted subcategory.

ALTER TABLE categories
    ADD COLUMN parent_id UUID REFERENCES categories(id) ON DELETE CASCADE;

-- The flat UNIQUE on name is too restrictive for a hierarchy
-- (e.g. "Other" could exist under several parents).
ALTER TABLE categories DROP CONSTRAINT categories_name_key;

-- Enforce uniqueness per scope instead:
--   • top-level names are unique among themselves
--   • subcategory names are unique within their parent
CREATE UNIQUE INDEX categories_unique_toplevel_name
    ON categories(name)
    WHERE parent_id IS NULL;

CREATE UNIQUE INDEX categories_unique_subcat_name
    ON categories(name, parent_id)
    WHERE parent_id IS NOT NULL;

CREATE INDEX idx_categories_parent ON categories(parent_id);
