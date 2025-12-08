{
  perSystem = {config, ...}: let
    inherit (config) craneLib src commonArgs cargoArtifacts;
  in {
    checks = {
      inherit (config.packages) bore-cli;

      bore-cli-clippy = craneLib.cargoClippy (
        commonArgs
        // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        }
      );

      bore-cli-doc = craneLib.cargoDoc (
        commonArgs
        // {
          inherit cargoArtifacts;
        }
      );

      bore-cli-fmt = craneLib.cargoFmt {
        inherit src;
      };

      bore-cli-nextest = craneLib.cargoNextest (
        commonArgs
        // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
        }
      );
    };
  };
}
