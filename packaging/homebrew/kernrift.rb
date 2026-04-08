# typed: false
# frozen_string_literal: true

class Kernrift < Formula
  desc "Self-hosted systems language compiler for kernel development"
  homepage "https://kernrift.org"
  version "2.4.0"
  license "MIT"

  BASE = "https://github.com/Pantelis23/KernRift/releases/latest/download"

  on_macos do
    on_intel do
      url "#{BASE}/krc-macos-x86_64"
      sha256 "PLACEHOLDER_MACOS_X86_64_SHA256"
    end

    on_arm do
      url "#{BASE}/krc-macos-arm64"
      sha256 "PLACEHOLDER_MACOS_ARM64_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "#{BASE}/krc-linux-x86_64"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end

    on_arm do
      url "#{BASE}/krc-linux-arm64"
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"
    end
  end

  def install
    # The downloaded file is the krc binary itself
    krc_name = stable.url.split("/").last
    bin.install krc_name => "krc"

    # Download the kr runner script
    kr_url = "#{BASE}/kr"
    curl_download kr_url, to: buildpath/"kr"
    bin.install "kr"
    chmod 0755, bin/"kr"

    # Download standard library modules
    std_dir = share/"kernrift/std"
    std_dir.mkpath

    %w[string io math fmt mem vec map color fb fixedpoint font memfast widget].each do |mod|
      raw_url = "https://raw.githubusercontent.com/Pantelis23/KernRift/main/std/#{mod}.kr"
      curl_download raw_url, to: std_dir/"#{mod}.kr"
    end
  end

  def caveats
    <<~EOS
      Standard library installed to:
        #{share}/kernrift/std/

      If krc cannot find the stdlib, set:
        export KR_STDLIB=#{share}/kernrift/std
    EOS
  end

  test do
    (testpath/"hello.kr").write <<~KR
      fn main() {
          print("hello from kernrift");
      }
    KR
    system bin/"krc", "hello.kr", "-o", "hello.krbo"
    assert_predicate testpath/"hello.krbo", :exist?
  end
end
