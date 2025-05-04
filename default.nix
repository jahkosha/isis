with import <nixpkgs> {};
{
  sdlEnv = stdenv.mkDerivation {
    name = "isis";
    shellHook = ''
      export RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=${mold}/bin/mold"
      export LIBCLANG_PATH="${llvmPackages.libclang.lib}/lib"
      export LD_LIBRARY_PATH="${lib.makeLibraryPath [stdenv.cc.cc udev alsa-lib xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr libxkbcommon wayland vulkan-loader libGL xorg.libxcb dbus]}"
    '';
    buildInputs = [
      rustup rust-analyzer rustfmt mold
      pkg-config
      xorg.libxcb
      dbus

      # aubio
      clang

      # macroquad
      # TODO
    ];
  };
}
