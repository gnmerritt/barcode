#!/bin/bash
cd "$(dirname "$0")"

REMOTE=batista.local
opponent=${1:-barcode-checkpoint2}

rsync -Calpr src/ ${REMOTE}:~/sources/barcode/src/
rsync -Calpr Cargo* ${REMOTE}:~/sources/barcode/

# give the build 3 seconds and then launch VNC here while SSH is open
( sleep 5 ; vnc-viewer -Scaling 175 ${REMOTE}:5900 ) &

ssh ${REMOTE} << EOF
  cd ~/sources/barcode/
  cargo build --release --target=i686-pc-windows-gnu
  cp target/i686-pc-windows-gnu/release/barcode.exe ~/.scbw/bots/barcode/AI/barcode.exe

  ~/.pyenv/versions/bw-docker/bin/scbw.play \
    --bots "barcode" "${opponent}"
EOF
