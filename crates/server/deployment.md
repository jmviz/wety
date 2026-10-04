# Server deployment

Build from the repository root using the pinned Rust toolchain:

```sh
cargo build --locked --release -p server
```

Deploy `target/release/server` together with an explicitly selected graph artifact.
Set `WETY_GRAPH_PATH` to its absolute path. The binary embeds the selected language
reference and POS table fingerprint; a graph built with different tables is rejected.
Keep the graph manifest, run receipt, `Cargo.lock`, source revision, build features,
and toolchain version with a deployment release.

For the application's existing TLS mode, set `WETY_ENVIRONMENT=production`,
`WETY_CERT_PATH`, and `WETY_KEY_PATH`. Certificates, domain routing, firewall policy
and process supervision are deployment-specific. The service binds port 3000 unless `WETY_PORT` is set.
The client is built/deployed separately from `client/`. It depends on `/resolve`
and on language codes in the API, so update the server before `main` deploys a
client that uses them; an open tab notices a new graph (409) and reloads.

The previous Oracle/PyTorch instructions are in git history
(`git show c472620:server/deployment.md`). The processor now uses Candle; that
old PyTorch build is not part of the server deployment.
