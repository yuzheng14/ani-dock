-- Add migration script here
-- store anime self
CREATE TABLE IF NOT EXISTS anime (
  id TEXT PRIMARY KEY NOT NULL,
  -- global unique key (including episode)
  sn INTEGER NOT NULL UNIQUE,
  -- anime's image
  cover TEXT NOT NULL,
  -- anime's name, including season
  name TEXT NOT NULL,

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL
) WITHOUT ROWID, STRICT;

CREATE TABLE IF NOT EXISTS series (
  id TEXT PRIMARY KEY NOT NULL,
  -- maybe `本篇` or dubbed or movie
  name TEXT NOT NULL,

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL,

  anime_id TEXT NOT NULL
    REFERENCES anime(id)
    ON DELETE CASCADE
    ON UPDATE RESTRICT,

  UNIQUE (anime_id, name)
) WITHOUT ROWID, STRICT;

CREATE TABLE IF NOT EXISTS episode (
  id TEXT PRIMARY KEY NOT NULL,
  sn INTEGER NOT NULL UNIQUE,
  cover TEXT NOT NULL,
  episode INTEGER NOT NULL,

  series_id TEXT NOT NULL
    REFERENCES series(id)
    ON DELETE CASCADE
    ON UPDATE RESTRICT,

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL
) WITHOUT ROWID, STRICT;

CREATE TABLE IF NOT EXISTS download_queue (
  id TEXT PRIMARY KEY NOT NULL,
  downloaded INTEGER NOT NULL DEFAULT 0
    CHECK (downloaded IN (0, 1)),

  episode_id TEXT NOT NULL UNIQUE
    REFERENCES episode(id)
    ON DELETE CASCADE
    ON UPDATE RESTRICT,

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL

) WITHOUT ROWID, STRICT;

CREATE INDEX IF NOT EXISTS idx_download_queue_downloaded
ON download_queue(downloaded);

CREATE TABLE IF NOT EXISTS anime_cover (
  id TEXT PRIMARY KEY NOT NULL,
  -- url of this cover image, for distinct
  url TEXT NOT NULL,
  -- url image data
  bytes BYTE NOT NULL,
  mime_type TEXT NOT NULL

  create_at TEXT NOT NULL,
  update_at TEXT NOT NULL
);
