"""Expose generated browser assets through explicit output groups."""

load("@rules_rust_wasm_bindgen//:defs.bzl", "RustWasmBindgenInfo")

def _asset(context):
    info = context.attr.source[RustWasmBindgenInfo]
    return [
        DefaultInfo(files = depset([info.wasm], transitive = [info.js])),
        OutputGroupInfo(javascript = info.js, webassembly = depset([info.wasm])),
    ]

asset = rule(
    implementation = _asset,
    attrs = {"source": attr.label(providers = [RustWasmBindgenInfo], mandatory = True)},
)
