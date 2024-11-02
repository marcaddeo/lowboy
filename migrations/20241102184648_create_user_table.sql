CREATE TABLE IF NOT EXISTS user (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT NOT NULL,
    byline TEXT NOT NULL,
    avatar TEXT NOT NULL,
    UNIQUE(email)
)
