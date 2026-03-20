class ArcCli < Formula
  desc "High-performance agentic CLI framework in Rust"
  homepage "https://github.com/Ashutosh0x/arc-cli"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/Ashutosh0x/arc-cli/releases/download/v1.0.0/arc-v1.0.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_arm do
      url "https://github.com/Ashutosh0x/arc-cli/releases/download/v1.0.0/arc-v1.0.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Ashutosh0x/arc-cli/releases/download/v1.0.0/arc-v1.0.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_arm do
      url "https://github.com/Ashutosh0x/arc-cli/releases/download/v1.0.0/arc-v1.0.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "arc"
  end

  test do
    assert_match "arc", shell_output("#{bin}/arc --version")
  end
end
