# Glacier Stream

A Rust CLI for streaming data to/from glacier.

# Facts

All other constraints aside, each Glacier upload worker can send data at
about 100Mb/s. So a 4-worker upload should upload at 400Mb/s.
