# chdef-wasm

The WebAssembly binding over [chdef](../chdef), built by
[`bindings/js`](../../bindings/js) into the npm package
`@shortarrow/chdef`.

**Using JavaScript or TypeScript?** `npm install @shortarrow/chdef` — this crate
is
the source of that package, not something to depend on.

**Writing Rust that targets `wasm32`?** Depend on `chdef` itself. It is
pure Rust and compiles to WebAssembly as it stands; the types here are
shaped for JavaScript, not for you.

## Building it

```sh
cd bindings/js && node build.mjs
```

That produces both targets the npm package ships — `bundler` for Vite and
friends, `nodejs` for Node — and needs
[wasm-pack](https://drager.github.io/wasm-pack/) and the
`wasm32-unknown-unknown` target.

## License

MIT OR Apache-2.0.
