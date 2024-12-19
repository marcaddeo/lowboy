-- Create token table.
CREATE TABLE IF NOT EXISTS token (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    secret TEXT NOT NULL,
    expiration DATETIME NOT NULL,
    FOREIGN KEY(user_id) REFERENCES lowboy_user(id)
);
