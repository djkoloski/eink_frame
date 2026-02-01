cargo build --target aarch64-unknown-linux-gnu --profile deploy \
&& sftp david@rpi-z2w-1.local << EOF
  put config.json
  put resources/graphics.rkyv resources/graphics.rkyv
  put target/aarch64-unknown-linux-gnu/deploy/eink_frame eink_frame
EOF
