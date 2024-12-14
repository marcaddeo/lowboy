-- Create email table.
CREATE TABLE IF NOT EXISTS email (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    address TEXT NOT NULL UNIQUE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY(user_id) REFERENCES lowboy_user(id)
);

-- Add admin email.
INSERT INTO email (id, user_id, address, verified)
VALUES (1, 1, 'admin@example.com', true);
