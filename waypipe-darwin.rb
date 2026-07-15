class WaypipeDarwin < Formula
  desc "Proxy for Wayland clients with macOS transport support"
  homepage "https://github.com/J-x-Z/waypipe-darwin"
  url "https://github.com/J-x-Z/waypipe-darwin/archive/refs/tags/v0.11.0-darwin.1.tar.gz"
  version "0.11.0-darwin.1"
  sha256 "5d1d0f1c384a1ca0fc7ef492ca73055343842fde4856cdbb77a7c2a2836cd6fa"
  head "https://github.com/J-x-Z/waypipe-darwin.git", branch: "main"

  depends_on "bindgen" => :build
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
    assert_match "waypipe 0.11.0", output
    assert_match "lz4: true", output
    assert_match "zstd: true", output
  end
end
