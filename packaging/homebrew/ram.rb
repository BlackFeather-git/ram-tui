class Ram < Formula
  desc "Lightweight, aesthetic zero-dependency real-time terminal memory monitor"
  homepage "https://github.com/BlackFeather-git/ram-tui"
  url "https://github.com/BlackFeather-git/ram-tui/archive/v0.7.0-rc.2.tar.gz"
  sha256 "REPLACE_WITH_RELEASE_SHA256"
  license "MIT"

  def install
    bin.install "ram"
    bash_completion.install "completions/ram.bash" => "ram"
    zsh_completion.install "completions/_ram" => "_ram"
    fish_completion.install "completions/ram.fish" => "ram.fish"
  end

  test do
    assert_match "ram v#{version}", shell_output("#{bin}/ram --version")
    system "#{bin}/ram", "--once", "--json"
  end
end
