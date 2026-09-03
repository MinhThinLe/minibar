{ pkgs }: rec {
    pkgConfigLibs = with pkgs; [
        libxkbcommon
        wayland
        dbus
    ];

    pkgConfigPath = pkgs.lib.strings.concatStringsSep ":" (
        builtins.map (lib: "${pkgs.lib.getDev lib}/lib/pkgconfig/") pkgConfigLibs
    );

    libraryPaths = pkgs.lib.makeLibraryPath pkgConfigLibs;
}
