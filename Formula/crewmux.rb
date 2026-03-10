class Crewmux < Formula
  desc "Multi-agent orchestration for tmux-powered teams"
  homepage "https://github.com/crewmux/client"
  head "https://github.com/crewmux/client.git", branch: "main"

  depends_on "rust" => :build
  depends_on "tmux"

  def install
    system "cargo", "install", *std_cargo_args(root: libexec)

    installed_bins = Dir[libexec/"bin/*"]
    odie "No executable found in #{libexec}/bin" if installed_bins.empty?

    bin.install installed_bins.first => "crewmux"
  end

  def caveats
    <<~EOS
      CrewMux needs at least one agent CLI on your PATH at runtime:
        - claude
        - codex

      tmux is installed automatically as a Homebrew dependency.
    EOS
  end

  test do
    assert_match "Usage: crewmux", shell_output("#{bin}/crewmux --help")
    assert_match "Start a new AI team session", shell_output("#{bin}/crewmux team --help")
  end
end
