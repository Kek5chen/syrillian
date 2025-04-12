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

        packages."windows-cross" = let
          winPkgs = import nixpkgs {
            system = system;
            crossSystem = {
              config = "x86_64-w64-mingw32";
            };
          };
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

          buildInputs = with winPkgs; [
            openssl
            (assimp.overrideAttrs (old: {
              outputs = [ "out" ];
            
              cmakeFlags = old.cmakeFlags ++ [
                "-DCMAKE_INSTALL_PREFIX=${placeholder "out"}"
                "-DASSIMP_BUILD_TESTS=OFF"
                "-DASSIMP_WARNINGS_AS_ERRORS=OFF"
                "-DBUILD_SHARED_LIBS=OFF"
              ];

              buildInputs = [ zlib ]; 
            }))
            windows.mingw_w64_pthreads
          ];

          LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
          #CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-L native=${winPkgs.assimp}/lib";
        };
      }
    );
}
