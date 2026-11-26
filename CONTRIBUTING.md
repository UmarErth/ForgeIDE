# Contributing to Forge IDE

Thanks for helping make Forge IDE better. Issues and pull requests are welcome.

## Before you begin

- Search existing issues before opening a duplicate.
- For a larger change, open an issue first so we can agree on the direction.
- Keep pull requests focused and describe the user-visible change.

## Build locally

Install Rust, Node.js 22 or later, and the native desktop dependencies for your operating system. Then run:

```sh
npm ci
cargo check
npm run build
```

To build a desktop package, install Tauri CLI v2 and follow the platform commands in the README.

## Pull requests

Include a clear summary, the reason for the change, and any platform-specific notes. Do not commit API keys, credentials, generated `target` files, or `node_modules`.
