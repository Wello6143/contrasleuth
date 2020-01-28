DELETE FROM inventory WHERE datetime(expiration_time, 'unixepoch') <= datetime('now')
