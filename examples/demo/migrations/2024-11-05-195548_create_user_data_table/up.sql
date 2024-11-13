-- Create user_data table.
CREATE TABLE IF NOT EXISTS user_data (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE,
    name TEXT NOT NULL,
    avatar TEXT,
    byline TEXT
);

-- Add admin user data.
INSERT INTO user_data (id, user_id, name)
VALUES (1, 1, 'Admin Istrator');
