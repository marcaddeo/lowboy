-- Create user_profile table.
CREATE TABLE IF NOT EXISTS user_profile (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE,
    name TEXT NOT NULL,
    avatar TEXT,
    byline TEXT,
    FOREIGN KEY(user_id) REFERENCES user(id)
);

-- Add admin user profile.
INSERT INTO user_profile (id, user_id, name)
VALUES (1, 1, 'Admin Istrator');
