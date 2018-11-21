# Hydroxide OS
> An experimental operating system kernel written in Rust.

## Building Hydroxide
Please always use a recent nightly build of Rust.

Dependencies:
- git
- A working nightly Rust toolchain (rustup is your friend)
- rustfmt-preview (`rustup component add rustfmt-preview`)
- bootimage (`rustup run nightly cargo install bootimage`)

### Setting up the build environment
```bash

# If you don't use 2FA or public-key authentication
git clone https://github.com/TheKernelCorp/Hydroxide.git hydroxide

# If you use 2FA and/or public-key authentication
git clone git@github.com:TheKernelCorp/Hydroxide.git hydroxide

# Enter the directory
cd hydroxide

# Setup the build environment
#
# The following command is only needed if you
# intend to modify the Hydroxide code.
./scripts/setup-dev-env
```

### Building only
> `bootimage build --release`

### Building and running
> Boot the kernel in qemu-system-x86_64:   
> `bootimage run --release`

## Thanks

Special thanks to [Philipp Oppermann][phil-opp] and the [Rust OSDev][rust-osdev] team for their excellent crates!

[phil-opp]: https://github.com/phil-opp
[rust-osdev]: https://github.com/rust-osdev