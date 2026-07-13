class WaypipeDarwin < Formula
  desc "Proxy for Wayland clients with macOS transport support"
  homepage "https://github.com/J-x-Z/waypipe-darwin"
  url "https://github.com/J-x-Z/waypipe-darwin.git", branch: "main"
  version "0.11.0-darwin"
  head "https://github.com/J-x-Z/waypipe-darwin.git", branch: "main"

  depends_on "rust" => :build
  depends_on "lz4"
  depends_on "zstd"

  def install
    ENV.prepend_path "PATH", Formula["rust"].opt_bin
    ENV.prepend_path "PKG_CONFIG_PATH", Formula["lz4"].opt_lib/"pkgconfig"
    ENV.prepend_path "PKG_CONFIG_PATH", Formula["zstd"].opt_lib/"pkgconfig"
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
