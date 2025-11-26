{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust Toolchain
    cargo
    rustc
    rust-analyzer
    clippy
    rustfmt

    # Build System Tools
    pkg-config

    # GStreamer and Glib dependencies
    glib
    gst_all_1.gstreamer
    gst_all_1.gst-plugins-base
    gst_all_1.gst-plugins-good
    gst_all_1.gst-plugins-bad
    gst_all_1.gst-plugins-ugly
    pipewire

    # Video encoding libraries often needed by plugins
    x264
  ];

  # Usually nix handles PKG_CONFIG_PATH automatically for buildInputs,
  # but sometimes explicitly setting it helps if using custom dev shells.
  shellHook = ''
    echo "Entering GStreamer-RS development shell..."
    export GST_PLUGIN_SYSTEM_PATH_1_0="${pkgs.lib.makeSearchPathOutput "lib" "lib/gstreamer-1.0" (with pkgs.gst_all_1; [
      gstreamer
      gst-plugins-base
      gst-plugins-good
      gst-plugins-bad
      gst-plugins-ugly
    ] ++ [ pkgs.pipewire ])}"
  '';
}
