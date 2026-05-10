-- Add an "ignored" flag to categories.
-- Transactions whose category has ignored = true are excluded from dashboard
-- totals and analytics aggregations.
ALTER TABLE categories ADD COLUMN ignored BOOLEAN NOT NULL DEFAULT false;
