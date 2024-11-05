-- Create user table.
CREATE TABLE IF NOT EXISTS user (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    username TEXT,
    password TEXT,
    access_token TEXT,
    name TEXT NOT NULL,
    byline TEXT,
    avatar TEXT
);

-- Add "admin" user (password: hunter42).
INSERT INTO user (id, email, username, password, name)
VALUES (1, 'admin@example.com', 'admin', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw', 'Admin Istrator');
