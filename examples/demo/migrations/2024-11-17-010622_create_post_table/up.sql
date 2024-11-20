-- Create post table.
CREATE TABLE IF NOT EXISTS post (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES user(id)
);
