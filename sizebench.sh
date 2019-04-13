#!/bin/sh

set -eux

INPUT=$(mktemp)
OUTPUT=$(mktemp --directory)

cleanup() {
  rm -f $INPUT
  rm -r $OUTPUT
}

trap cleanup EXIT

cargo build --release
BIN=target/release/hjq

unxz --stdout data/citylots.json.xz > $INPUT

time $BIN index --input=$INPUT --data-dir=$OUTPUT
du -sh $OUTPUT
