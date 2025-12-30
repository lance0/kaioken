# Homebrew formula for kaioken
# To install: brew tap lance0/kaioken && brew install kaioken
# Or: brew install lance0/kaioken/kaioken

class Kaioken < Formula
  desc "High-performance HTTP load testing tool with real-time TUI"
  homepage "https://github.com/lance0/kaioken"
  version "1.1.1"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-macos-aarch64.tar.gz"
      sha256 "d87cdb91c3e377545a776c94aa9f2831fe62c7a055c1c3ddb2e98c61eb521cca"
    else
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-macos-x86_64.tar.gz"
      sha256 "b2b27a4fcc1dad5b5f43d182f1afa4cf6198a8cdaf78f7c9a0c8ab8297ab4102"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-linux-aarch64.tar.gz"
      sha256 "5913ea9952c80a64b2e4b41dfb789e516406938d35c8f6ad8beb9d7f18728210"
    else
      url "https://github.com/lance0/kaioken/releases/download/v#{version}/kaioken-linux-x86_64.tar.gz"
      sha256 "b27300f4c5254683ec5edbb8553b6b85f75a341497abde45b329215afdfb2c3b"
    end
  end

  def install
    bin.install "kaioken"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/kaioken --version")
  end
end
