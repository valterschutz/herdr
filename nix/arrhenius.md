# Standalone Arrhenius build

Arrhenius runs x86-64 Linux with glibc 2.34 and no Nix. The default Nix
package uses Nix-store paths and a newer glibc, so copying its executable
alone does not work. The development shell includes `cargo-zigbuild` and
the x86-64 musl Rust target to build a standalone executable instead:

```sh
nix develop
export CARGO_TARGET_DIR="$HOME/.cache/herdr-arrhenius-target"
LIBGHOSTTY_VT_OPTIMIZE=ReleaseFast cargo zigbuild --release --locked \
  --target x86_64-unknown-linux-musl
```

The binary is at
`~/.cache/herdr-arrhenius-target/x86_64-unknown-linux-musl/release/herdr`.
Keep a copy under `~/synced/misc/herdr-arrhenius/<source-commit>/`.

This is an opt-in portable build, not a change to the default package or its
runtime behavior. Static musl avoids the cluster's glibc version constraint
and requires no bundled libraries, but differs from the default glibc build;
verify the resulting executable on Arrhenius as well as locally.

Use the Arrhenius/ssh-cluster skills for transfer and verification. Transfer to
`~/bin/herdr.new`, verify its SHA-256 and `--version`, then rename it over
`~/bin/herdr` (never write over the running executable). Replacing the file
does not upgrade an already running server; do not stop or restart sessions
without the user's approval.
