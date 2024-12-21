-- Create lowboy_user table.
CREATE TABLE IF NOT EXISTS lowboy_user (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password TEXT,
    access_token TEXT
    CHECK((password IS NULL AND access_token IS NOT NULL) OR (access_token IS NULL AND password IS NOT NULL))
);

-- Add admin user (password: hunter42).
INSERT INTO lowboy_user (id, username, password)
VALUES (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw');
