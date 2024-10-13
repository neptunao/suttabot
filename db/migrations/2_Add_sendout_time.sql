CREATE TABLE sendout_times
(
    id INTEGER PRIMARY KEY,
    subscription_id INTEGER NOT NULL,
    sendout_time INTEGER NOT NULL, -- Store time as integer (e.g., 0 for 00:00, 60 for 01:00)
    FOREIGN KEY (subscription_id) REFERENCES subscription(id)
);

CREATE INDEX idx_subscription_chat_id ON subscription(chat_id);

CREATE INDEX idx_subscription_is_enabled ON subscription(is_enabled);

CREATE INDEX idx_sendout_times_sendout_time ON sendout_times(sendout_time);

-- Insert sendout_time = 500 for every active chat_id, which is 05:00 UTC and 08:00 MSK
INSERT INTO sendout_times (subscription_id, sendout_time)
SELECT id, 500
FROM subscription;
