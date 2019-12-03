DELETE FROM inventory WHERE datetime(expiration_time, 'unixepoch') <= datetime('now', 'unixepoch')
INSERT INTO inventory VALUES (?, ?, ?, ?)