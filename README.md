# `eink_frame`

## Setup

You'll need to be able to cross-compile for ARM so this can be run on your
target device. I'm targeting a Raspberry Pi Zero 2 W, so this setup will list
the steps for that specifically:

1.  Install the Rust target for `aarch64-unknown-linux-gnu`:
    `rustup target add aarch64-unknown-linux-gnu`
2.  [Download the appropriate toolchain for your development host][toolchain]
    -   For cross-compilation from x86_64 Linux, you'll want to download
        "x86_64 Linux hosted cross toolchains" >
        "AArch64 GNU/Linux target (aarch64-none-linux-gnu)"
3. Configure your build by writing `.cargo/config.toml`:

```toml
[target.aarch64-unknown-linux-gnu]
linker = "<TOOLCHAIN_PATH>/bin/aarch64-none-linux-gnu-gcc"

# Setting target-cpu gives the compiler additional optimization information.
# Most embedded devices have very weak CPUs, so we need all the optimization we
# can get. You can list the available CPUs with
# `rustc --print=target-cpus --target=aarch64-unknown-linux-gnu`.
#
# The Raspberry Pi Zero 2 W has an ARM Cortex-A53 CPU.
rustflags = ["-C", "target-cpu=cortex-a53"]
```

### Performance tuning

For best performance, you'll also need to increase the size of the SPI device buffer. The device buffer is limited to 4096 by default, forcing the image transfer to be split into multiple consecutive transfers and thus kernel roundtrips.

To check the size of the SPI buffer:

```
cat /sys/module/spidev/parameters/bufsiz
```

To increase the size of the SPI buffer:

```
sudo nano /boot/firmware/cmdline.txt
```

and append

```
spidev.bufsiz=192000
```

then reboot the device.

## Host development

To develop on host, `cargo run` and then `cargo run -p emulator` to start the
emulator.

[toolchain]: https://developer.arm.com/downloads/-/arm-gnu-toolchain-downloads
