{
    description = "A status bar that aims to do less";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    };

    outputs = { self, nixpkgs }: {
        packages = builtins.mapAttrs (system: pkgs: {
            minibar = pkgs.callPackage ./nix/package.nix { };
            default = self.packages.${pkgs.stdenv.hostPlatform.system}.minibar;
        }) nixpkgs.legacyPackages;

        devShells = builtins.mapAttrs (system: pkgs: {
            default =
                let
                    shared = import ./nix/shared.nix { inherit pkgs; };
                in
                pkgs.mkShell {
                    nativeBuildInputs =
                        with pkgs;
                        [
                            cargo
                            clippy
                            pkg-config
                            rust-analyzer
                            rustc
                            rustfmt
                        ]
                        ++ shared.pkgConfigLibs;

                    LD_LIBRARY_PATH = shared.libraryPaths;
                    PKG_CONFIG_PATH = shared.pkgConfigLibs;
                };
        }) nixpkgs.legacyPackages;
    };
}
