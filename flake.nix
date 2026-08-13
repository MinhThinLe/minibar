{
    description = "A very basic flake";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    };

    outputs = inputs: {
        devShells = builtins.mapAttrs (system: pkgs: {
            default =
                let
                    libraries = with pkgs; [
                        wayland
                        libxkbcommon
                    ];
                in
                pkgs.mkShell {
                    packages = with pkgs; [
                        pkg-config
                    ];
                    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
                    PKG_CONFIG_PATH = pkgs.lib.strings.concatStringsSep ":" (
                        builtins.map (lib: "${pkgs.lib.getDev lib}/lib/pkgconfig") libraries
                    );
                };
        }) inputs.nixpkgs.legacyPackages;
    };
}
