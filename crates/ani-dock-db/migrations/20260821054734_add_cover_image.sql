-- Add migration script here
CREATE TABLE IF NOT EXISTS cover_image (
  id TEXT PRIMARY KEY NOT NULL,
  -- url of this cover image, for distinct
  url TEXT NOT NULL UNIQUE,
  -- url image data
  bytes BLOB NOT NULL,
  mime_type TEXT NOT NULL,

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL
) WITHOUT ROWID, STRICT;

ALTER TABLE anime ADD COLUMN cover_id TEXT
REFERENCES cover_image(id)
ON DELETE RESTRICT
ON UPDATE RESTRICT;

ALTER TABLE episode ADD COLUMN cover_id TEXT
REFERENCES cover_image(id)
ON DELETE RESTRICT
ON UPDATE RESTRICT;
