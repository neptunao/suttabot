-- Add sendout_count field for message-count-based donation reminders
-- This tracks how many daily suttas have been sent to each user

-- Add sendout_count: Counter that increments each time a daily sutta is sent
-- Initialize to 14 so first donation message comes at 15 (if period is 15)
ALTER TABLE subscription ADD COLUMN sendout_count INTEGER NOT NULL DEFAULT 14;
