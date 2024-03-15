CREATE TABLE subscription
(
    id INTEGER PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    is_enabled INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
