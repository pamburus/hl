@0xb4c004ef430e6d9f;

# Root is the root entity of index file.
struct Root {
    source @0 :SourceFile;
}

# Source contains metadata of scanned source log file.
struct SourceFile {
    size @0 :UInt64;
    sha256 @1 :Data;
    path @2 :Text;
    modified :group{
        sec @3 :Int64;
        nsec @4 :UInt32;
    }
    index @5 :Index;
    blocks @6 :List(SourceBlock);
}

# Block is an information about a part of source log file.
struct SourceBlock {
    offset @0 :UInt64;
    size @1 :UInt32;
    index @2 :Index;
    chronology @3 :Chronology;
    hash @4 :Hash;
}

# Index holds index information of a block or a whole file.
struct Index {
    flags @0 :UInt64;
    lines :group{
        valid @1 :UInt64;
        invalid @2 :UInt64;
    }
    timestamps :group {
        min :group {
            sec @3 :Int64;
            nsec @4 :UInt32;
        }
        max :group {
            sec @5 :Int64;
            nsec @6 :UInt32;
        }
    }
}

# Chronology holds information about ordering of log messages by timestamp in a SourceBlock.
# It can be used to effectively iterate over log messages in chronological order.
struct Chronology {
    # Each item in a `bitmap` holds 64 bits for 64 source lines.
    # Bit value 0 means the corresponding line goes chronologically after previous line.
    # Bit value 1 means the corresponding line does not go chronologically after previous line and there is a jump value for it in a jump table.
    # Offset in a jump table for the first line in item N of bitmap can be found in offsets.jumps[N].
    # Each next line referenced by the same item in bitmap uses the same offset in jump table if it has bit value 0, or an offset with added 1 if it has bit value 1.
    # Offset in a SourceBlock bytes for a first line referenced by bitmap item N can be found in offsets.bytes[N].
    # Each next line referenced by the same item in bitmap can be located in the SourceBlock bytes at offset of previous line + length of previous line if it has bit value 0 in bitmap, or at offset specified in a jump table if it has bit value 1.
    bitmap @0 :List(UInt64);
    # Group `offsets` holds offsets in SourceBlock bytes and in a `jumps` table for each 64th line.
    offsets :group {
        bytes @1 :List(UInt32);
        jumps @2 :List(UInt32);
    }
    # Field `jumps` holds offsets in a SourceBlock bytes for lines which breaks chronological order.
    jumps @3 :List(UInt32);
}

# HashAlgorithm is an algorithm used to calculate data hash.
enum HashAlgorithm {
    sha256 @0;
    gxHash64 @1;
    wyHash @2;
}

# Hash is a hash of some data.
struct Hash {
    algorithm @0 :HashAlgorithm;
    value @1 :Data;
}

# Various flags.
const flagLevelError :UInt64    = 0x0000000000000008;
const flagLevelWarning :UInt64  = 0x0000000000000020;
const flagLevelInfo :UInt64     = 0x0000000000000100;
const flagLevelDebug :UInt64    = 0x0000000000001000;
const flagLevelTrace :UInt64    = 0x0000000000004000;
const flagLevelMask :UInt64     = 0x000000000000FFFF;
const flagUnsorted :UInt64      = 0x0000000000010000;
const flagHasTimestamps :UInt64 = 0x0000000000020000;
const flagBinary :UInt64        = 0x8000000000000000;
