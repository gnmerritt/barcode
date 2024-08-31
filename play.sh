#!/bin/bash
cd "$(dirname "$0")"

REMOTE=batista.local

rsync -Calpr src/ ${REMOTE}:~/sources/barcode/src/

# give the build 3 seconds and then launch VNC here while SSH is open
( sleep 5 ; vnc-viewer -Scaling 175 ${REMOTE}:5900 ) &

ssh ${REMOTE} << 'EOF'
  cd ~/sources/barcode/
  cargo build --release --target=i686-pc-windows-gnu
  cp target/i686-pc-windows-gnu/release/barcode.exe ~/.scbw/bots/barcode/AI/barcode.exe

  export PATH=${PATH}:~/.pyenv/versions/bw-docker/bin/  
  scbw.play --bots "barcode" "barcode-checkpoint"
EOF
