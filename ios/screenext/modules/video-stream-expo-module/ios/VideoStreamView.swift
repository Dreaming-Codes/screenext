import ExpoModulesCore
import UIKit

// This view will be used as a native component. Make sure to inherit from `ExpoView`
// to apply the proper styling (e.g. border radius and shadows).
class VideoStreamView: ExpoView {
  
  // REQUIRED for GStreamer glimagesink to draw
  override class var layerClass: AnyClass {
    return CAEAGLLayer.self
  }

  required init(appContext: AppContext? = nil) {
    super.init(appContext: appContext)
    clipsToBounds = true
    // Initialize the video-capable layer
    self.layer.masksToBounds = true
    // Initialize Rust GStreamer ONCE
    ios_stream_init()
  }
  
  override func layoutSubviews() {
    super.layoutSubviews()
  }
  
  deinit {
    ios_stream_stop()
  }
  
  func setPort(_ port: Int) {
      // Pass the UIView pointer (self) to Rust so it can overlay video
      // We bridge the pointer to void*
      let viewPointer = Unmanaged.passUnretained(self).toOpaque()
      ios_stream_start(viewPointer, UInt16(port))
  }
}
