{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs?ref=release-24.11";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        disable-graphics-settings = false;
    
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
      in with pkgs; rec {
        devShell = mkShell rec {
          buildInputs = [
            libxkbcommon
            libGL
            cmake

            rustPlatform.bindgenHook

            # Needed for static linking assimp in russimp-sys in russimp
            #stdenv.cc.cc.lib
            zlib.static

            # Dependency of openssl-sys
            pkg-config
            openssl.dev

            assimp
          ] ++ lib.lists.optionals (!disable-graphics-settings) [
            # WINIT_UNIX_BACKEND=wayland
            wayland

            ## WINIT_UNIX_BACKEND=x11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11

            ## To make Vulkan available
            vulkan-headers
            vulkan-loader
            vulkan-validation-layers
            vulkan-tools
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };
      });
}
