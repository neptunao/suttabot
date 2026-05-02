ALTER TABLE subscription ADD COLUMN announcements_enabled INTEGER NOT NULL DEFAULT 1;
ALTER TABLE subscription ADD COLUMN news_onboarded INTEGER NOT NULL DEFAULT 1;

CREATE TABLE news_broadcast (
    slug TEXT PRIMARY KEY NOT NULL,
    broadcast_at TEXT NOT NULL,
    recipient_count INTEGER NOT NULL,
    triggered_by INTEGER NOT NULL,
    version TEXT NOT NULL
);
