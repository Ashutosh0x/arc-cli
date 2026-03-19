class ArcCli < Formula
  desc "ARC - Agent for Rapid Coding"
  homepage "https://github.com/Ashutosh0x/arc-cli"
  url "https://github.com/Ashutosh0x/arc-cli/archive/refs/tags/v0.5.0.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/arc-cli")
  end

  test do
    assert_match "ARC — Agent for Rapid Coding", shell_output("#{bin}/arc --version")
  end
end
