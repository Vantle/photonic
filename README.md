<p align="center">
  <a href="https://photonic.vantle.org">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="book/logo/dark.svg">
      <img src="book/logo/light.svg" alt="Photonic" width="112" height="112">
    </picture>
  </a>
</p>

<h1 align="center">Photonic</h1>

<p align="center">Write the rules. Explore every outcome.</p>

<p align="center">
  <a href="https://photonic.vantle.org"><b>Read the webbook</b></a>
  &nbsp;·&nbsp;
  <a href="https://photonic.vantle.org/sandbox.html"><b>Open the sandbox</b></a>
  &nbsp;·&nbsp;
  <a href="library/README.md">Standard library</a>
  &nbsp;·&nbsp;
  <a href="theorem/README.md">Theorems</a>
  &nbsp;·&nbsp;
  <a href="document/README.md">Documentation</a>
</p>

<br>

Photonic is a language of rules. A program is data and the rules that change it. The runtime explores every configuration those rules can reach, and Prism checks each result exactly: a target is reached, unreachable or unknown.

```
Light
[Light] Red
[Light] Green
[Light] Blue
```

Three rules can consume `Light`, so this program has three futures. The runtime follows all of them and finds four configurations: `Light`, then `Red`, `Green` or `Blue`.

The standard library is written in Photonic, and its theorems are proved by running them in every case. The runtime is Rust, and it also runs in the browser: every example in the webbook is live.

## Try it

The [sandbox](https://photonic.vantle.org/sandbox.html) runs Photonic in your browser: write a program, run it, and explore every configuration it reaches, with nothing to install. Its links carry the whole program, so you can share what you write.

To work from a checkout, Bazel is the only thing to install. It fetches the pinned compilers and dependencies.

Save the program above as `light.wave` and run it:

```sh
bazel run -c opt //command:photonic -- run "$PWD/light.wave"
```

Serve the webbook from a checkout, then open http://127.0.0.1:8080:

```sh
bazel run -c opt //toolchain/browser:serve
```

## Develop

Every build also checks formatting and lints, and warnings are errors.

```sh
bazel test -c opt //...
bazel run -c opt //:format
bazel run -c opt //:link
```

After changing an example in the webbook, record its runs again with `bazel run -c opt //book:record`; `//book:record.check` fails until the record matches the page.
