{
  imports = [
    ./rust-toolchain.nix
    ./crane-lib.nix
    ./args.nix
    ./artifacts.nix
  ];

  perSystem = {config, ...}: let
    inherit (config) craneLib commonArgs commonArgsNative cargoArtifacts cargoArtifactsNative;
  in {
    packages = {
      bore-cli = craneLib.buildPackage (commonArgs
        // {
          inherit cargoArtifacts;
        });

      bore-cli-native = craneLib.buildPackage (commonArgsNative
        // {
          inherit cargoArtifactsNative;
        });
    };
  };
}
