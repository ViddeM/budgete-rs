-- Remove the default color value introduced in 0001 so that callers must
-- supply an explicit color when creating a category.
ALTER TABLE categories ALTER COLUMN color DROP DEFAULT;
