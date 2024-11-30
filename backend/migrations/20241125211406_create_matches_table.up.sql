CREATE TYPE status AS ENUM ('x', 'o', 'tied', 'pending');

CREATE TABLE IF NOT EXISTS matches (
    state status NOT NULL DEFAULT 'pending',
    id UUID PRIMARY KEY NOT NULL,
    snapshot JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Create a function to update the updated_at column
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create a trigger that calls the function before any update
CREATE TRIGGER update_updated_at
BEFORE UPDATE ON matches
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();