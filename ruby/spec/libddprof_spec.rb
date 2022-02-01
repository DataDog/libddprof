# frozen_string_literal: true

require "tmpdir"
require "fileutils"

RSpec.describe Libddprof do
  describe "version constants" do
    it "has a version number" do
      expect(Libddprof::VERSION).to_not be nil
    end

    it "has an upstream libddprof version number" do
      expect(Libddprof::LIB_VERSION).to_not be nil
    end
  end

  describe "binary helper methods" do
    let(:temporary_directory) { Dir.mktmpdir }

    before do
      allow(ENV).to receive(:[]).and_call_original
      allow(ENV).to receive(:[]).with("LIBDDPROF_VENDOR_OVERRIDE").and_return(temporary_directory)
    end

    after do
      FileUtils.remove_dir(temporary_directory)
    end

    context "when no binaries are available in the vendor directory" do
      describe ".no_binaries?" do
        it { expect(Libddprof.no_binaries?).to be true }
      end

      describe ".available_binaries" do
        it { expect(Libddprof.available_binaries).to be_empty }
      end

      describe ".pkgconfig_folder" do
        it { expect(Libddprof.pkgconfig_folder).to be nil }
      end
    end

    context "when binaries are available in the vendor directory" do
      before do
        Dir.mkdir("#{temporary_directory}/386-freedos")
        Dir.mkdir("#{temporary_directory}/mipsel-linux")
      end

      describe ".no_binaries?" do
        it { expect(Libddprof.no_binaries?).to be false }
      end

      describe ".available_binaries" do
        it { expect(Libddprof.available_binaries).to eq ["386-freedos", "mipsel-linux"] }
      end

      context "for the current platform" do
        let(:pkgconfig_folder) { "#{temporary_directory}/#{Gem::Platform.local}/some/folder/containing/the/pkgconfig/file" }

        before do
          FileUtils.mkdir_p(pkgconfig_folder)
          File.open("#{pkgconfig_folder}/ddprof_ffi.pc", "w") {}
        end

        describe ".pkgconfig_folder" do
          it "returns the folder containing the pkgconfig file" do
            expect(Libddprof.pkgconfig_folder).to eq pkgconfig_folder
          end
        end
      end

      context "but not for the current platform" do
        describe ".pkgconfig_folder" do
          it { expect(Libddprof.pkgconfig_folder).to be nil }
        end
      end
    end
  end
end
