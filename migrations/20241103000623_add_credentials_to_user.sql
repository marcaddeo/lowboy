ALTER TABLE user ADD COLUMN username TEXT DEFAULT '' NOT NULL; -- @TODO This should be unique.
ALTER TABLE user ADD COLUMN password TEXT DEFAULT '' NOT NULL;

UPDATE user SET username = "marc", PASSWORD = "$argon2id$v=19$m=19456,t=2,p=1$VE0e3g7DalWHgDwou3nuRA$uC6TER156UQpk0lNQ5+jHM0l5poVjPA1he/Tyn9J4Zw" WHERE id = 1;
