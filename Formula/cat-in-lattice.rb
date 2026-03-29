class CatInLattice < Formula
  desc "Ghostty companion pane with pixel art cat, banners, and mini-games"
  homepage "https://github.com/NomaDamas/Cat-In-Lattice"
  url "https://github.com/NomaDamas/Cat-In-Lattice/archive/refs/tags/v0.1.0.tar.gz"
  # sha256 will be filled after release
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "cat-in-lattice", shell_output("#{bin}/cat-in-lattice --version")
  end
end
