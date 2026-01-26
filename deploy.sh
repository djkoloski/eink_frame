cargo build --target aarch64-unknown-linux-gnu --profile deploy \
&& scp target/aarch64-unknown-linux-gnu/deploy/eink_frame david@rpi-z2w-1.local:/home/david
