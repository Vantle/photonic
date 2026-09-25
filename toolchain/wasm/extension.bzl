"""Checksum-pinned official binding tools, matching rules_rs 0.0.110."""

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

def _binding(context):
    for platform, digest in {
        "aarch64-apple-darwin": "2011b2027c3dc68616c3d3265be95f37e12fef8ef21e2dcf0011275da58d19bb",
        "x86_64-apple-darwin": "b2a465b4538f0bf11685e296ac181c92deb45434cfc849c87a4b418c79214ac6",
        "aarch64-unknown-linux-musl": "26bec5b1e7ba80e0fd65e5d9f4c88c304f04d9c08d2a4da32ab31af8e2fee8eb",
        "x86_64-unknown-linux-musl": "b391448c4926ac4b11425a6752484d85164e72489d97804461d5e868c643b88a",
    }.items():
        archive = "wasm-bindgen-0.2.105-" + platform
        http_archive(
            name = platform,
            urls = ["https://github.com/wasm-bindgen/wasm-bindgen/releases/download/0.2.105/" + archive + ".tar.gz"],
            sha256 = digest,
            strip_prefix = archive,
            build_file_content = 'exports_files(["wasm-bindgen"])',
        )
    return context.extension_metadata(reproducible = True)

binding = module_extension(implementation = _binding)
