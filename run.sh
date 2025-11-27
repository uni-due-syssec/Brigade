#!/usr/bin/env sh

echo "Start"

./target/release/brigade -l -r -s="100000" --end-block="100005" --replay-config ./config/replay_config.json

echo "Finished"
