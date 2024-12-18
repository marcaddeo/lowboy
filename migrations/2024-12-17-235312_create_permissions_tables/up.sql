-- Create role table.
CREATE TABLE IF NOT EXISTS role (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

-- Create permission table.
CREATE TABLE IF NOT EXISTS permission (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

-- Create user_role table.
CREATE TABLE IF NOT EXISTS user_role (
    user_id INTEGER REFERENCES lowboy_user(id),
    role_id INTEGER REFERENCES role(id),
    PRIMARY KEY (user_id, role_id)
);

-- Create user_permission table.
CREATE TABLE IF NOT EXISTS user_permission (
    role_id INTEGER REFERENCES role(id),
    permission_id INTEGER REFERENCES permission(id),
    PRIMARY KEY (role_id, permission_id)
);

-- Add anonymous role.
INSERT INTO role (name)
VALUES ('anonymous');

-- Add authenticated role.
INSERT INTO role (name)
VALUES ('authenticated');

-- Add administrator role.
INSERT INTO role (name)
VALUES ('administrator');

-- Add admin user to administrator role.
INSERT INTO user_role (user_id, role_id)
VALUES (1, (SELECT id FROM role WHERE name = 'administrator'));
