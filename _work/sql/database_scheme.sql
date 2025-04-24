CREATE TABLE "uris" (
    "uri_uuid"    TEXT PRIMARY KEY,
    "url"         TEXT NOT NULL,
    "scheme"      TEXT NOT NULL,
    "host"        TEXT,
    "path"        TEXT,
    "live_status" TEXT,
    "title"       TEXT,
    "auto_descr"  TEXT,
    "man_descr"   TEXT,
    "crea_user"   TEXT DEFAULT 'system',
    "crea_time"   TEXT,
    "modi_user"   TEXT DEFAULT 'system',
    "modi_time"   TEXT
);

CREATE TABLE "uri_import" (
    "url_raw"     TEXT NOT NULL,
    "sharedate"   TEXT DEFAULT '2021-01-01'
);

CREATE TABLE "uri_dates" (
    "uri_uuid"   TEXT NOT NULL,
    "sharedate"  TEXT NOT NULL
);

----------------------
