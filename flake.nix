{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixpkgs-unstable";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        disable-graphics-settings = false;
    
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;
      in {
        devShell = with pkgs; mkShell rec {
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

            # WINIT_UNIX_BACKEND=x11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11

            # To make Vulkan available
            vulkan-headers
            vulkan-loader
            vulkan-validation-layers
            vulkan-tools
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
        };

        # I give up. I abandon this for now.
        packages."windows-cross" = let
          winPkgs = import nixpkgs {
            system = system;
            crossSystem = {
              config = "x86_64-w64-mingw32";
            };
          };
          assimpCross = winPkgs.assimp.overrideAttrs (old: {
              outputs = [ "out" ];
            
              cmakeFlags = [ # This also disregards the built of assimp tools
                "-DCMAKE_INSTALL_PREFIX=${placeholder "out"}"
                "-DASSIMP_BUILD_TESTS=OFF"
                "-DASSIMP_WARNINGS_AS_ERRORS=OFF"
                "-DBUILD_SHARED_LIBS=OFF"
              ];

              buildInputs = [ winPkgs.zlib ]; 
            });
        in 
        winPkgs.rustPlatform.buildRustPackage rec {
          pname = "syrillian";
          version = "0.1.2";

          src = ./.;

          useCargoFetchVendor = true;
          cargoHash = "sha256-NIhNXbueWXrYmPUrPOZqmyaZONalzJqfhraxmDcOOOc=";

          #nativeBuildInputs = [
          #  pkgs.pkg-config
          #];

          nativeBuildInputs = [
            pkgs.pkg-config
          ];

          buildInputs = with winPkgs; [
            winPkgs.openssl.dev
            pkgs.rustPlatform.bindgenHook

            zlib
            assimpCross
          ];

          cargoBuildFlags = [
            "--example necoarc"
          ];

          #LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = [
            #"-L" "native=${winPkgs.windows.mcfgthreads}/lib"
            #"-L" "native=${(pkgs.pkgsCross.mingwW64.zstd.override {
            #    enableStatic = true;
            #  }).out}/lib"

            #"-L" "native=${(winPkgs.zstd.override {
            #  enableStatic = true;
            #}).out}/lib"
            #"-l" "static=zstd"

            "-C" "target-feature=+crt-static"
          ];
          #PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

        };
      }
    );
}
