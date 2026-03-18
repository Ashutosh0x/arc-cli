# ARC CLI: Installation Guide

ARC CLI is distributed as a source-available Rust crate. To install and compile ARC locally for your system architecture, follow these steps.

## Prerequisites

ARC requires a modern Rust toolchain.
- **Rust**: Version `1.85.0` or higher.
- **Cargo**: Installed alongside Rust.
- **Git**: For version control integrations (`arc-worktree` and `arc-hooks`).
- **C/C++ Build Tools**: Required for compiling `tokio-uring` (Linux) or some cryptography libraries.

**Install Rust via rustup:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Option 1: Cargo Install (Recommended)

To compile and install the CLI globally on your machine directly from the official repository:

```bash
cargo install --git https://github.com/Ashutosh0x/arc-cli.git --locked
```

This will place the compiled `arc` executable into your `~/.cargo/bin` directory, which should be in your system `$PATH`.

## Option 2: Clone and Build from Source

For developers who wish to modify the agent framework or contribute:

1. **Clone the repository:**
   ```bash
   git clone https://github.com/Ashutosh0x/arc-cli.git
   cd arc-cli
   ```

2. **Run the build script:**
   ARC is optimized with strict LTO and 1 codegen unit. Compiling for release takes longer but provides maximum execution speed.
   ```bash
   cargo build --release --workspace
   ```

3. **Symlink to Path (Linux / macOS):**
   ```bash
   sudo ln -s $(pwd)/target/release/arc /usr/local/bin/arc
   ```

   **Add to Path (Windows PowerShell):**
   ```powershell
   $env:Path += ";$pwd\target\release"
   ```

## Option 3: Docker (Sandboxed Environments)

If you prefer to run ARC entirely sandboxed without installing Rust natively:

```bash
docker pull ashutosh0x/arc-cli:latest
docker run -v $(pwd):/workspace -it ashutosh0x/arc-cli arc chat
```

## Verifying the Installation

Verify that the CLI is properly linked within your terminal by checking the internal diagnostics command:

```bash
arc doctor
```

The doctor command will verify presence of necessary dependencies (`git`, `docker`, API Keys) and print any remaining configuration steps.