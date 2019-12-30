CREATE TABLE IF NOT EXISTS inventory (
    blake2b BLOB PRIMARY KEY,
    payload BLOB,
    nonce INTEGER,
    expiration_time INTEGER
)