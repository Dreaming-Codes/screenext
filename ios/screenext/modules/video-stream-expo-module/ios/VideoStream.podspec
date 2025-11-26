require 'json'

package = JSON.parse(File.read(File.join(__dir__, '..', 'package.json')))

Pod::Spec.new do |s|
  s.name           = 'VideoStream'
  s.version        = package['version']
  s.summary        = package['description']
  s.description    = package['description']
  s.license        = package['license']
  s.author         = package['author']
  s.homepage       = package['homepage']
  s.platforms      = {
    :ios => '15.1',
    :tvos => '15.1'
  }
  s.swift_version  = '5.9'
  s.source         = { git: 'https://github.com/Dreaming-Codes/screenext/ios/video-stream-expo-module' }
  s.static_framework = true

  s.dependency 'ExpoModulesCore'

  # Swift/Objective-C compatibility

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'SWIFT_OBJC_BRIDGING_HEADER' => '$(PODS_TARGET_SRCROOT)/ios/StreamHandler.h',
    'OTHER_LDFLAGS' => '-force_load $(PODS_TARGET_SRCROOT)/ios/libios_stream_handler.a -framework GStreamer -framework AVFoundation -framework CoreMedia -framework CoreAudio -framework VideoToolbox -framework AudioToolbox',
    'LIBRARY_SEARCH_PATHS' => '$(PODS_TARGET_SRCROOT)/ios',
    'HEADER_SEARCH_PATHS' => '$(PODS_TARGET_SRCROOT)/ios',
    'FRAMEWORK_SEARCH_PATHS' => '"$(PODS_TARGET_SRCROOT)/ios/Frameworks" "/Library/Frameworks"'
  }

  s.script_phases = [
    {
      :name => 'Build Rust Library',
      :script => 'bash "${PODS_TARGET_SRCROOT}/build_rust.sh"',
      :execution_position => :before_compile
    }
  ]

  s.source_files = "**/*.{h,m,mm,swift,hpp,cpp}"
end
