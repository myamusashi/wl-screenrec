{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  installShellFiles,
  libdrm,
  ffmpeg,
  wayland,
  src,
}:

rustPlatform.buildRustPackage {
  pname = "wl-screenrec";
  version = "0.2.0";

  inherit src;

  cargoLock = {
    lockFileContents = builtins.readFile ../Cargo.lock;
    outputHashes = {
      "ffmpeg-sys-next-9.0.0" = "sha256-sDSJ2l+1Nh2t87zR5PNGmw4Pa+4n2Jnd6o48wC7jLyM=";
    };
  };

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    installShellFiles
  ];

  buildInputs = [
    wayland
    libdrm
    ffmpeg
  ];

  doCheck = false;

  postInstall = lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd wl-screenrec \
      --bash <($out/bin/wl-screenrec --generate-completions bash) \
      --fish <($out/bin/wl-screenrec --generate-completions fish) \
      --zsh <($out/bin/wl-screenrec --generate-completions zsh)
  '';

  meta = {
    description = "High performance wlroots screen recording, featuring hardware encoding";
    homepage = "https://github.com/russelltg/wl-screenrec";
    license = lib.licenses.asl20;
    platforms = lib.platforms.linux;
    mainProgram = "wl-screenrec";
  };
}
