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
          # Tests are run via separate checks (bore-cli-nextest)
          doCheck = false;
        });

      bore-cli-native = craneLib.buildPackage (commonArgsNative
        // {
          inherit cargoArtifactsNative;
          # Tests are run via separate checks (bore-cli-nextest)
          doCheck = false;
        });
    };
  };
}
