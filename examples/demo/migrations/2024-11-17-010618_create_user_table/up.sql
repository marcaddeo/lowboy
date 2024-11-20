-- Create user_data table.
CREATE TABLE IF NOT EXISTS user (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    lowboy_user_id INTEGER NOT NULL UNIQUE,
    name TEXT NOT NULL,
    avatar TEXT,
    byline TEXT,
    FOREIGN KEY(lowboy_user_id) REFERENCES lowboy_user(id)
);

-- Add admin user data.
INSERT INTO user (id, lowboy_user_id, name)
VALUES (1, 1, 'Admin Istrator');
