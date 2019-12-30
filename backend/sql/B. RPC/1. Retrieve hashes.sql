SELECT blake2b FROM inventory WHERE datetime(expiration_time, 'unixepoch') > datetime('now', 'unixepoch')
