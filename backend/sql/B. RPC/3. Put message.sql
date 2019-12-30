DELETE FROM inventory WHERE datetime(expiration_time, 'unixepoch') <= datetime('now', 'unixepoch')
INSERT OR IGNORE INTO inventory VALUES (?, ?, ?, ?)