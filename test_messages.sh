#!/bin/bash
cd ~/src/amoeba/gromnie
rm -f /tmp/gromnie-test.log
RUST_LOG=info ./target/debug/gromnie 2>&1 | tee /tmp/gromnie-test.log &
PID=$!
sleep 15
kill $PID
echo "===== ALL MESSAGES RECEIVED ====="
grep "Parsed as S2CMessage\|Unhandled S2CMessage\|Unknown message\|OrderedGameEvent" /tmp/gromnie-test.log
