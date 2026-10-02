class WaypipeDarwin < Formula
  desc "Proxy for Wayland clients with macOS transport support"
  homepage "https://github.com/J-x-Z/waypipe-darwin"
  url "https://github.com/J-x-Z/waypipe-darwin/archive/refs/tags/v0.11.2-darwin.1.tar.gz"
  version "0.11.2-darwin.1"
  sha256 "c273ee104a00a7b91d8ba4a7a7df0ff79b0b0ce1e724985685bb86b5c2cd5fa6"
  license "GPL-3.0-or-later"
  head "https://github.com/J-x-Z/waypipe-darwin.git", branch: "main"

  depends_on "bindgen" => :build
  depends_on "pkgconf" => :build
  depends_on "rust" => :build
  depends_on "lz4"
  depends_on "zstd"

  def install
    ENV.prepend_path "PATH", formula_opt_bin("bindgen")
    ENV.prepend_path "PATH", formula_opt_bin("rust")
    ENV.prepend_path "PKG_CONFIG_PATH", formula_opt_lib("lz4")/"pkgconfig"
    ENV.prepend_path "PKG_CONFIG_PATH", formula_opt_lib("zstd")/"pkgconfig"
    system "cargo", "install", *std_cargo_args,
           "--no-default-features", "--features", "lz4,zstd"
  end

  test do
    output = shell_output("#{bin}/waypipe --version")
    assert_match "waypipe 0.11.2", output
    assert_match "lz4: true", output
    assert_match "zstd: true", output
  end
end
