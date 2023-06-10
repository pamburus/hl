# Sorting workflows

## Regular file

### Current implementation

1. Main thread
    1. Filter blocks (by timestamp and level filters)
    2. Sort blocks in chronological order by first record timestamp
2. Pusher thread
    1. Read and push blocks one by one
3. Worker threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records
    4. If there are no records, go to (1)
    5. Format records into a preallocated buffer
    6. Collect formatted records ranges
    7. Push the formatted block
4. Merger thread
    1. Pull next formatted block
    2. Put the block into workspace
    3. Sort all blocks in a workspace by next record timestamp
    4. Copy the first record of the first block
    5. If its timestamp is >= than timestamp of recenly pulled formatted block, go to (1), else go to (3)

## Named pipe or standard input

### Current implementation

1. Reader thread
    1. Read input splitting into blocks
    2. Archive each block and store into memory
2. Indexer threads
    1. Index each block
3. Pusher thread
    1. Filter blocks (by timestamp and level filters)
    2. Sort blocks in chronological order by first record timestamp
    3. Read and push blocks one by one
4. Worker threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records
    4. If there are no records, go to (1)
    5. Format records into a preallocated buffer
    6. Collect formatted records ranges
    7. Push the formatted block
5. Merger thread
    1. Pull next formatted block
    2. Put the block into workspace
    3. Sort all blocks in a workspace by next record timestamp
    4. Copy the first record of the first block
    5. If its timestamp is >= than timestamp of recenly pulled formatted block, go to (1), else go to (3)

### Desired implementation

1. Reader thread
    1. Read input splitting into blocks
2. Parser threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records (if no filter, (3)..(6) may be skipped)
    4. If there are no records, go to (1)
    5. Copy records to a new block
    6. Collect ranges of the records
    7. Archive block and store it into memory
    8. Push the metadata of the block down to the pipeline
    9. Go to (1)
3. Pusher thread
    1. Collect all incoming blocks
    2. Filter blocks (by timestamp and level filters) (? - duplicates 2.3)
    3. Sort blocks in chronological order by first message timestamp
    4. Read and push blocks one by one
4. Worker threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records (? - duplicates 2.3)
    4. If there are no records, go to (1)
    5. Format records into a preallocated buffer
    6. Collect formatted records ranges
    7. Push the formatted block
5. Merger thread
    1. Pull next formatted block
    2. Put the block into workspace
    3. Sort all blocks in a workspace by next record timestamp
    4. Copy the first record of the first block
    5. If its timestamp is >= than timestamp of recenly pulled formatted block, go to (1), else go to (3)

## Compressed file

### Desired implementation

1. Main thread
    1. Filter blocks (by timestamp and level filters)
    2. Sort blocks in chronological order by first record timestamp
    3. Evaluate block lifetimes along the chronological stream
2. Reader thread
    1. Iterate over the filtered and sorted blocks
        1. If next block does not go in order
            1. For each block before it
                1. Drop it if filtered out by (1.1)
                2. Push it to the archiver threads otherwise
        2. Read the block
        3. Push it to the worker threads
3. Archiver threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records (if no filter, (3)..(4) may be skipped)
    4. Copy records to a new block
    5. If there are no records left, go to (1)
    6. Archive the block and store into memory
    7. Go to (1)
4. Worker threads
    1. Take next incoming block and split it into lines
    2. Parse lines as records
    3. Filter records (? - duplicates 3.3)
    4. If there are no records, go to (1)
    5. Format records into a preallocated buffer
    6. Collect formatted records ranges
    7. Push the formatted block
5. Merger thread
    1. Pull next formatted block
    2. Put the block into workspace
    3. Sort all blocks in a workspace by next record timestamp
    4. Copy the first record of the first block
    5. If its timestamp is >= than timestamp of recenly pulled formatted block, go to (1), else go to (3)


