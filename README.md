# Keyboard firmware for the Pinky4
Recently, I bought a
[Pinky4](https://github.com/tamanishi/Pinky4/tree/master) keyboard
kit, and I wanted to write some firmware in Rust for it. For the
micro-controller, I'm using the Sparkfun Pro micro rp2040, which will
have lots of storage, and a nice JST connector to a display, which
I'll find a way to put onto the board.

I'll be using these crates:
- https://github.com/TeXitoi/keyberon
- https://github.com/rtic-rs/rtic
- https://github.com/rp-rs/rp-hal (as well as the bsp for the sparkfun rp2040 pro micro)

.. and more.

I was also looking into embassy, but it seemed harder to get into
because they have an entire ecosystem of crates.

# Building and Running
This project requires nightly rust(due to RTIC). Refer to [the
flake](./flake.nix) to get all the necessary tools. Use:
```
nix develop
```
to enter the dev environment.


Do:
```
cargo build
```

to build. 

When running, you need to reset the RP2040 Pro Micro by holding the
boot button, then press the reset button once, and releasing the boot
button. The board should show up as a USB drive. Then you can do:
```
cargo run
```

to flash the firmware onto the board(it uses the: `elf2uf2-rs` tool for the usb upload).
