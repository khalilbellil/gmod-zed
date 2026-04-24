# gmod-zed

Garry's Mod / GLua support for the [Zed editor](https://zed.dev).

A port of the "ethos" of William Venner's
[`vscode-glua-enhanced`](https://github.com/WilliamVenner/vscode-glua-enhanced)
to Zed's extension model. It provides:

- A `GLua` language bound to `.lua` files.
- Tree-sitter grammar (`tree-sitter-grammars/tree-sitter-lua`) with highlights,
  indents, brackets, outline, text objects, and injections tuned for GMod
  (adds `continue`, GMod globals, constants, and LuaJIT `ffi.cdef`).
- Snippets ported from the VS Code extension (`for-i`, `for-pairs`,
  `for-ipairs`, full keyword set) plus `hook.Add`, `net.Receive`,
  `concommand.Add`, `timer.Create`.
- A Rust/WASM extension that boots
  [LuaLS](https://github.com/LuaLS/lua-language-server) and wires in the
  community-maintained
  [`glua-api-snippets`](https://github.com/luttje/glua-api-snippets) addon
  so completions, hovers, signatures, and go-to-definition line up with
  the Garry's Mod wiki.

If you want the editor-UI bits of the VS Code extension (bytecode heatmap,
VMT/PNG previews, globals optimizer, etc.) see [`MIGRATION.md`](./MIGRATION.md)
for what is and isn't portable into Zed today.

## Install

### As a dev extension

1. Install Rust via [`rustup`](https://www.rust-lang.org/tools/install)
   (Homebrew Rust won't work; Zed requires rustup-installed toolchains).
2. Clone this repo.
3. In Zed, open the Extensions page (`zed: extensions`).
4. Click **Install Dev Extension** and pick this directory.

Zed will `cargo build --release --target wasm32-wasip1` the crate for you
and register the language, grammar, snippets, and LSP server.

### Published (not yet)

This extension isn't yet on the Zed extension registry. Publishing is a PR
to [`zed-industries/extensions`](https://github.com/zed-industries/extensions)
once the port has had some real-world use. See "Open issues" in
[`MIGRATION.md`](./MIGRATION.md).

## Coexistence with the official Lua extension

Zed already has a first-party [Lua extension](https://github.com/zed-extensions/lua)
that also claims `*.lua`. Both extensions can be installed at once, but only
one wins per buffer. For GMod projects the cleanest setup is to force the
`GLua` language for that workspace:

```json
// .zed/settings.json in your GMod project root
{
  "file_types": {
    "GLua": ["**/*.lua"]
  }
}
```

Alternatively, uninstall the Lua extension if you only work on Garry's Mod
Lua.

## Configuration

The LSP is registered as `LuaLS (GLua)`. You can override anything in your
settings file (`zed: open settings`):

```json
{
  "lsp": {
    "LuaLS (GLua)": {
      "binary": {
        "path": "/custom/path/to/lua-language-server",
        "arguments": []
      },
      "settings": {
        "Lua": {
          "diagnostics": {
            "globals": ["MyAddonGlobal"]
          },
          "hint": {
            "enable": false
          }
        }
      }
    }
  }
}
```

Your `settings` are deep-merged over the defaults the extension ships.

If you'd rather bring your own LuaLS install (e.g. via
[Mason](https://github.com/williamboman/mason.nvim),
[asdf](https://asdf-vm.com), a system package, or a manual download), set
`lsp."LuaLS (GLua)".binary.path` and the extension will skip its managed
install. `lua-language-server` already on your `$PATH` is also picked up
automatically.

## What's inside

```
.
├── extension.toml               # manifest: grammar, LSP, snippets
├── Cargo.toml                   # Rust / wasm crate
├── src/lib.rs                   # LuaLS launcher + GLua addon installer
├── languages/
│   └── glua/
│       ├── config.toml          # GLua language binding
│       ├── highlights.scm       # + GMod globals/constants
│       ├── brackets.scm
│       ├── indents.scm
│       ├── outline.scm
│       ├── textobjects.scm
│       └── injections.scm
├── snippets/
│   └── glua.json
├── README.md
├── MIGRATION.md                 # source-feature → Zed mapping
├── LICENSE
└── .gitignore
```

## Credits

- [`WilliamVenner/vscode-glua-enhanced`](https://github.com/WilliamVenner/vscode-glua-enhanced)
  — the original VS Code extension that defined the feature surface ported
  here.
- [`luttje/glua-api-snippets`](https://github.com/luttje/glua-api-snippets)
  — the wiki-derived LuaCats annotations that give LuaLS GMod awareness.
- [`LuaLS/lua-language-server`](https://github.com/LuaLS/lua-language-server)
  — the language server doing all the real work.
- [`zed-extensions/lua`](https://github.com/zed-extensions/lua) — the
  reference Lua extension whose tree-sitter queries, indent rules, and LSP
  boot logic this port builds on.
