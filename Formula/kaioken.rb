# Homebrew formula for kaioken
# To install: brew tap lance0/kaioken && brew install kaioken
# Or: brew install lance0/kaioken/kaioken

class Kaioken < Formula
  desc "High-performance HTTP load testing tool with real-time TUI"
  homepage "https://github.com/lance0/kaioken"
  version "1.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-macos-aarch64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_MACOS_ARM64"
    else
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-macos-x86_64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_MACOS_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-linux-aarch64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX_ARM64"
    else
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-linux-x86_64.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX_X86_64"
    end
  end

  def install
    bin.install "kaioken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/kaioken --version")
  end
end
