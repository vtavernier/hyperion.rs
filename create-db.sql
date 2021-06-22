-- Script for creating a new empty settings database
-- Adds PRIMARY and FOREIGN key constraints compared to the original

CREATE TABLE instances (
	instance INTEGER NOT NULL PRIMARY KEY,
	friendly_name TEXT NOT NULL,
	enabled INTEGER NOT NULL DEFAULT 0,
	last_use TEXT NOT NULL
);

CREATE TABLE auth (
	user TEXT NOT NULL PRIMARY KEY,
	password BLOB NOT NULL,
	token BLOB NOT NULL,
	salt BLOB NOT NULL,
	comment TEXT,
	id TEXT,
	created_at TEXT NOT NULL,
	last_use TEXT NOT NULL
);

CREATE TABLE meta (
	uuid TEXT NOT NULL PRIMARY KEY,
	created_at TEXT NOT NULL
);

CREATE TABLE settings (
	type TEXT NOT NULL,
	config TEXT NOT NULL,
	hyperion_inst INTEGER,
	updated_at TEXT NOT NULL,
	CONSTRAINT settings_pk PRIMARY KEY (type, hyperion_inst)
	FOREIGN KEY (hyperion_inst) REFERENCES instances(instance)
);
