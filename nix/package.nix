{
    lib,
    rustPlatform,
    pkgs,
}:

rustPlatform.buildRustPackage (
    finalAttrs:
    let
        shared = import ./shared.nix { inherit pkgs; };
    in
    {
        pname = "minibar";
        version = "0.1.0";

        src = lib.cleanSource ../.;

        cargoDeps = rustPlatform.importCargoLock {
            lockFile = ../Cargo.lock;
            outputHashes = {
                "accesskit-0.22.0" = "sha256-pP9CyiV1zIONQ7vbl5MkMtilemSPrHaZ0c/SyR+lb0k=";
                "build_helpers-0.14.0" = "sha256-JwuEMw4xssYynHr+dQ8aMtZC1IuA/nri6ZPqizWRjxc=";
                "clipboard_macos-0.1.0" = "sha256-WO3JFbE+6ESRAfkxrnEFeZyGuhUHLOKOVHcGQyHwoK0=";
                "cosmic-client-toolkit-0.2.0" = "sha256-LUAmB+3+doRZOJbVURaIInaQuV/LXCKfoWHA28ihAMo=";
                "dpi-0.1.2" = "sha256-8r9O5RgVa8vxkPPYvr2aQiRdZ4isg7Jdnk8O5gQIr9k=";
                "smithay-clipboard-0.8.0" = "sha256-GojAFRbhJcP0Rpr+v9WOivgW9x38PZdeBWTbMhkDB3A=";
                "softbuffer-0.4.1" = "sha256-9Ret/nfieBFl4yJ9TddyWsSuS7sI4QAza/TZrxYMb+I=";
            };
        };

        nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
        ]
        ++ shared.pkgConfigLibs;

        buildInputs = shared.pkgConfigLibs;

        PKG_CONFIG_PATH = shared.pkgConfigPath;

        postInstall = ''
            wrapProgram $out/bin/minibar \
                --prefix LD_LIBRARY_PATH : ${shared.libraryPaths}
        '';

        meta = {
            mainProgram = "minibar";
        };
    }
)
