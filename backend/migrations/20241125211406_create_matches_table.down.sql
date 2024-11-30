-- Drop the trigger first
DROP TRIGGER IF EXISTS update_updated_at ON matches;

-- Drop the trigger function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop the table
DROP TABLE IF EXISTS matches;

-- Drop the enum type last (must be after table since table uses it)
DROP TYPE IF EXISTS status;