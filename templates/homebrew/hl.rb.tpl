class Humanlog < Formula
  desc ""
  homepage ""
  version "{{ version }}"
  bottle :unneeded

  if OS.mac?
    url "https://github.com/pamburus/hl/releases/download/{{ version }}/hl-macos.tar.gz"
    sha256 "{{ sha256 }}"
  end

  def install
    bin.install "hl"
  end
end
