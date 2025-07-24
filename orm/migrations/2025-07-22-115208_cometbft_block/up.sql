CREATE TABLE cometbft_block (
    id INT PRIMARY KEY,
    encoded_block VARCHAR NOT NULL,
    encoded_block_result VARCHAR NOT NULL,
    epoch INT NOT NULL
);