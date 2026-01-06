-- Add donation tracking fields to subscription table
-- SQLite doesn't support non-constant defaults in ALTER TABLE, so we use 0 and UPDATE

-- Add last_donation_reminder: Unix timestamp (UTC)
-- Default 0 is a placeholder; we UPDATE existing rows and explicitly set for new rows
ALTER TABLE subscription ADD COLUMN last_donation_reminder INTEGER NOT NULL DEFAULT 0;

-- Add donation_reminder_count: Counter for number of reminders sent
ALTER TABLE subscription ADD COLUMN donation_reminder_count INTEGER NOT NULL DEFAULT 0;

-- Initialize existing subscriptions to 14 days ago (UTCNOW - 1209600 seconds)
-- This ensures all users get donation reminder with their next daily message
UPDATE subscription SET last_donation_reminder = strftime('%s', 'now') - 1209600;
