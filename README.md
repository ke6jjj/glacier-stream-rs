# Glacier Stream

Glacier-stream is a Rust-based CLI for streaming data to/from glacier.

Glacier-stream allows you to stream data to and from an AWS Glacier vault
as a pipe utility (stdin/stdout) without having to use a filesystem to host
or store the data first. Use it much as you would a sequential/tape drive
in emergency situations, or where local storage simply isn't available for
your job. Example use cases include:

* Streaming a live filesystem snapshot/dump, such as `ufsdump` or `zfs send`,
  to Glacier for immediate archival.

* Sending a large archive to Glacier, but encrypting it first, without
  using extra intermediate storage.

* Streaming a backup from Glacier directly into a filesystem restoration
  utility in single-user mode with no local storage, such as `ufsrestore`
  or `zfs recv`.

# Usage

Glacier-stream has three modes:

* `up`: Stream data TO Glacier
* `down`: Stream data FROM Glacier
* `tree-hash`: Compute a "Tree Hash" to verify local data matches that in Glacier

In addition to chosing a mode, the utility will also need AWS credentials
to operate correctly when streaming.

## AWS Secrets Environment

In the streaming modes you will need appropriate AWS authorization tokens and
secrets available. To this end, the utility will obey the same secret loading
scheme as the standard AWS CLI utilities, namely:

* AWS environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` and
  `AWS_SESSION_TOKEN`.

* AWS credentials file (`~/.aws/credentials`)

* AWS config file (`~/.aws/config`)

It is beyond the scope of this README to describe how to set up the
environment. There should be plenty of information elsewhere on how to do so.

## Upload mode

## Download mode

## Tree Hash mode

# Facts

All other constraints aside, each Glacier upload worker can send data at
about 100Mb/s. So a 4-worker upload should upload at 400Mb/s.
