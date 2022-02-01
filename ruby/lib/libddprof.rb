# frozen_string_literal: true

require_relative "libddprof/version"

module Libddprof
  # Is this a no-op libddprof release without binaries?
  def self.no_binaries?
    available_binaries.empty?
  end

  def self.available_binaries
    File.directory?(vendor_directory) ? Dir.children(vendor_directory) : []
  end

  def self.pkgconfig_folder
    current_platform = Gem::Platform.local.to_s

    return unless available_binaries.include?(current_platform)

    pkgconfig_file = Dir.glob("#{vendor_directory}/#{current_platform}/**/ddprof_ffi.pc").first

    return unless pkgconfig_file

    File.absolute_path(File.dirname(pkgconfig_file))
  end

  private_class_method def self.vendor_directory
    ENV["LIBDDPROF_VENDOR_OVERRIDE"] || "#{__dir__}/../vendor/libddprof-#{Libddprof::LIB_VERSION}/"
  end
end
