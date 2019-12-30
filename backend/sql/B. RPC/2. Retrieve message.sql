SELECT payload, nonce, expiration_time FROM inventory WHERE blake2b = ? AND datetime(expiration_time, 'unixepoch') > datetime('now', 'unixepoch')
