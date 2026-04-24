# Migration: `vscode-glua-enhanced` → `gmod-zed`

Source: <https://github.com/WilliamVenner/vscode-glua-enhanced> (GPL-3.0).

This doc is the source-of-truth for what was ported, what was redesigned,
and what is not currently achievable in Zed.

## Executive assessment

The VS Code extension is a **fat in-editor runtime** — it downloads a
`gmod-wiki.json` blob and then drives VS Code's `CompletionItemProvider`,
`HoverProvider`, `SignatureHelpProvider`, `DocumentColorProvider`,
`DefinitionProvider`, `ReferenceProvider`, `CodeLensProvider`, and `Command`
APIs directly from JavaScript.

Zed's extension model is different: almost all "intelligence" features are
delivered through a **language server**, not from the extension process
itself. The Rust/WASM extension runtime only exposes:

- grammar registration,
- language configuration and tree-sitter queries,
- snippets,
- launching/managing a language server,
- `label_for_completion` / `label_for_symbol` polish on LSP-provided items,
- slash commands, context servers, and debug adapters (orthogonal features).

It exposes **no** APIs for hovers, completion items, signatures, color
decorators, inlay hints, code actions, or arbitrary UI, outside of what
the language server returns.

Therefore the faithful port of `vscode-glua-enhanced` is:

1. GLua language + grammar + tree-sitter queries + snippets (in-extension).
2. LuaLS as the LSP, pointed at the `glua-api-snippets` LuaCats annotations
   (in-extension).
3. Everything else either (a) gets a clean LSP-driven equivalent inside
   LuaLS, (b) becomes a feature request against LuaLS / Zed, or
   (c) is accepted as out-of-scope.

## Feature inventory (source extension)

### syntax / language config
- `contributes.languages`: `glua` id, `.lua` extension.
- `contributes.grammars`: TextMate grammar `syntaxes/lua.tmLanguage.json`
  (forked from `sumneko/vscode-lua`).
- `resources/language-configuration.json`: line/block comments, brackets,
  auto-closing pairs, surrounding pairs, indentation regexes.

### snippets
- `snippets/custom.json`: `for-i`, `for-pairs`, `for-ipairs`.
- `snippets/keywords.json`: one snippet per Lua keyword
  (`then`, `do`, `for`, `in`, `goto`, `::`, `if`, `elseif`, `else`,
  `while`, `repeat`, `continue`, `break`, `return`, `or`, `and`, `local`,
  `function`, `true`, `false`, `nil`, `NULL`, `_G`, `_VERSION`, `self`,
  `end`).

### completions (`src/completionProvider.js`, ~37 KB)
- Global autocompletion from a wiki JSON dump.
- Function argument / enum argument completion.
- Library table member completion (e.g. `hook.`, `net.`, `util.`).
- Workspace `lua/`, `models/`, `materials/`, `sound/` file browsers.
- Default `materials/flags16/`, `materials/icon16/`, `sound/` catalogs.
- NetworkVar discovery + completion.
- Net message discovery + completion.
- Local + global identifier completion from a JS-side Lua parser
  (`src/gluaparse.js`, `src/tokenizer.js`).

### hovers/docs (`src/hoverProvider.js`)
- Markdown-rendered wiki docs on identifiers.
- Notes / warnings / bugs / realm flags / deprecated / internal / new flags.
- "View Source" links into the GMod GitHub mirror for Lua-defined functions.
- Hover-to-see string length + cursor position.
- Hover-to-decode `\\xNN` byte sequences.

### diagnostics
- None (`vscode-glua-enhanced` doesn't lint; README recommends `glualint`).

### workspace / project scanning
- `GLuaParser.parseWorkspace()` walks all `.lua` files, parses them with
  the bundled `gluaparse`, indexes globals / locals / NetworkVars / net
  messages.
- "Find Globals" (`glua-enhanced.findGlobals`) dumps workspace globals.
- "Localize Global Calls" rewrites globals to `local X = X` at the top of
  the current file or every file in the workspace.

### commands
- `glua-enhanced.findGlobals`
- `glua-enhanced.bytecodeHeatmap`
- `glua-enhanced.optimizeGlobals`
- `glua-enhanced.optimizeGlobalsWorkspace`

### semantic analysis
- `gluaparse` Lua AST parser (bundled).
- Tokenizer for scope-aware identifier extraction.
- DefinitionProvider / ReferenceProvider wired on top.
- TypesProvider renders "type hints" as inlay-style decorators.
- ColorProvider adds a color picker on `Color(r, g, b, a)`.
- VMTProvider previews Valve Material Type text files as images.
- BytecodeHeatmapProvider shells out to `gluac` and colors hot spans.

### UI-specific VS Code features
- `contributes.iconThemes` (`GLua` icon theme for `.lua`, `.vmt`, `.vtf`,
  `.mdl`, `.vtx`, `.vvd`, `.phy`).
- `contributes.colors` (custom theme color `glua_enhanced.typeHints.color`).
- Status bar "Downloading GMod Wiki" spinner.
- Informational toast on first install / version bump.

### settings / config
- Stores `vscode-glua-enhanced-wiki-data2` + `vscode-glua-enhanced-wiki-date2`
  in VS Code's `globalState`.
- No user-facing `contributes.configuration` keys.

## Migration matrix

| # | Source feature | How it works in VS Code | Zed equivalent | Porting strategy | Risk / blocker | Priority | Status |
|--|--|--|--|--|--|--|--|
| 1 | `.lua` → GLua language registration | `contributes.languages` | `languages/glua/config.toml` + `path_suffixes = ["lua"]` | Done | Collides with official Lua extension; resolved via `file_types` | P0 | **Directly portable** |
| 2 | Lua TextMate grammar | `contributes.grammars` | Tree-sitter grammar registered in `extension.toml` + `highlights.scm` | Reuse `tree-sitter-grammars/tree-sitter-lua` (same grammar zed-extensions/lua uses) | GLua-only syntax (`continue`, `!=`, `&&`, `!`) isn't in the upstream grammar; highlighted via extra captures in `highlights.scm`; fine for highlight but parse errors will remain | P0 | **Portable with redesign** |
| 3 | Language configuration (comments, brackets, indentation) | JSON file | `languages/glua/config.toml` + `indents.scm` + `brackets.scm` | Done | — | P0 | **Directly portable** |
| 4 | Snippets | `contributes.snippets` | `snippets/glua.json` registered via `snippets = [...]` in `extension.toml` | Done (plus a few GMod bodies) | Zed only honours the first prefix per snippet — matches existing file | P0 | **Directly portable** |
| 5 | Wiki-driven completions | `CompletionItemProvider` + `gmod-wiki.json` | LSP completion from LuaLS + `glua-api-snippets` LuaCats annotations | Download addon in `install_glua_api_addon`; expose via `workspace.library` in LuaLS workspace configuration | LuaLS isn't _literally_ reading the same wiki JSON, but the annotations are generated from it and give equivalent coverage | P0 | **Requires external language server** |
| 6 | Signature help | `SignatureHelpProvider` | LSP `textDocument/signatureHelp` from LuaLS | Free from LuaLS once (5) works | — | P0 | **Requires external language server** |
| 7 | Hover docs | `HoverProvider` + wiki markdown | LSP `textDocument/hover` from LuaLS | Free from LuaLS once (5) works | Lacks the realm-flag emoji, bug/warning/note badges, and "View Source" link — annotations use plain LuaCats comments | P1 | **Portable with redesign** |
| 8 | Hover: string length / cursor position | Custom JS | No equivalent in Zed extension API | Accept as out of scope | Would require extension-side access to the active buffer selection, which Zed doesn't expose to extensions | P3 | **Not currently possible in Zed** |
| 9 | Hover: decode `\\xNN` byte sequences | Custom JS | No equivalent | Same as (8) | Same | P3 | **Not currently possible in Zed** |
| 10 | Goto definition / references | `DefinitionProvider` + `ReferenceProvider` | LSP `textDocument/definition` + `references` from LuaLS | Free from LuaLS | Works for workspace Lua; engine-defined symbols resolve into the addon files | P0 | **Requires external language server** |
| 11 | Global identifier discovery (workspace parse) | `GLuaParser.parseWorkspace` | LuaLS workspace diagnostics + completion | Free from LuaLS | LuaLS reports undefined globals; we pre-allow GMod globals in `diagnostics.globals` | P1 | **Portable with redesign** |
| 12 | "Find Globals" command | `glua-enhanced.findGlobals` | No user-invokable command API in Zed extensions | Drop, or re-implement as an LSP code lens in a future fork | P3 | **Not currently possible in Zed** |
| 13 | "Localize Global Calls" command | `globalsOptimizer.js` AST rewrite | Would need an LSP code action | Upstream-able to LuaLS as a code action plugin, or wrap as a separate CLI | P3 | **Not currently possible in Zed** |
| 14 | "Localize Global Calls (Workspace)" | Same | Same | Same | P3 | **Not currently possible in Zed** |
| 15 | Bytecode heatmap | `glua-enhanced.bytecodeHeatmap` + `gluac` + PNG rendering | No extension-side decoration / webview API in Zed | Drop; keep `gluac` as a standalone tool | P3 | **Not currently possible in Zed** |
| 16 | Color provider on `Color()` | `DocumentColorProvider` | LSP `textDocument/documentColor` | LuaLS doesn't publish document colors today; would need either an upstream LuaLS change or a tiny sidecar LSP that only provides document colors and runs alongside LuaLS | P2 | **Unknown, needs validation** |
| 17 | Inlay "type hints" (`typesProvider.js`) | Custom decoration | LSP inlay hints | LuaLS inlay hints are enabled in `language_server_workspace_configuration` (`Lua.hint.*`) | — | P1 | **Portable with redesign** |
| 18 | VMT / PNG previews (`vmtProvider.js`) | Custom hover markdown with embedded PNG | No image / rich-hover support via the extension API in Zed | Drop; document as separate viewer | P3 | **Not currently possible in Zed** |
| 19 | `contributes.iconThemes` (GLua file icons) | VS Code iconThemes | Zed icon themes are a separate extension type; Zed docs explicitly require shipping icon themes as distinct extensions, not bundled with languages | Split into a future `gmod-icons` extension | P2 | **Portable with redesign** |
| 20 | `contributes.colors` (`typeHints.color`) | VS Code theme color key | No extension-declared theme colors in Zed | Drop — inlay hint color is theme-driven in Zed | P3 | **Not currently possible in Zed** |
| 21 | First-install / update toast | VS Code `window.showInformationMessage` | No extension API for UI toasts | Drop | P3 | **Not currently possible in Zed** |
| 22 | `gmod-wiki.json` download + cache | `https.get(https://venner.io/...)` + `globalState` | Extensions can `zed::download_file`, but the extension would still need consumer APIs (hover, completion) to surface the data — which don't exist | Skip direct download; rely on `glua-api-snippets` LuaLS addon instead | P0 | **Portable with redesign** |
| 23 | Workspace file browsers (`models/`, `materials/`, `sound/`) in completions | Custom `CompletionItemProvider` that walks the worktree | LuaLS doesn't know about GMod asset paths; would need a companion LSP that completes string literals against the worktree | Propose as a future sidecar LSP in this repo | P3 | **Not currently possible in Zed** (needs new LSP) |
| 24 | Recommended companion `glualint` | External VS Code extension | `glualint` runs as a standalone linter; can be wired as a second language server via an extension today, but out of scope here | Track as follow-up | P2 | **Portable with redesign** |
| 25 | Debugging crash reports | — | — | n/a | — | — | — |

### Status legend

- **Directly portable** — exists in Zed's manifest-level configuration.
- **Portable with redesign** — the behaviour is achievable but the
  mechanism differs (LSP config, separate extension type, etc.).
- **Requires external language server** — delivered by LuaLS, not by this
  extension itself.
- **Not currently possible in Zed** — the extension API does not expose
  the hook needed. Either wait for Zed to add it, or build a sidecar LSP
  that surfaces the feature over LSP.
- **Unknown, needs validation** — depends on a third-party capability we
  haven't verified.

## Recommended target architecture

Hybrid: **grammar + language config + snippets shipped in-extension, all
semantic features delegated to LuaLS.**

Reasons:

- The extension API in Zed does not offer in-process completion / hover /
  code-action / color providers. Trying to re-implement the VS Code
  provider model in Rust/WASM would produce a dead-end codebase.
- LuaLS is mature, fast, and already the de facto Lua LSP; LuaLS+addon is
  what most GMod developers on Neovim/Helix already use.
- `luttje/glua-api-snippets` is auto-generated from the live Garry's Mod
  wiki — the same source that feeds the VS Code extension's
  `gmod-wiki.json`. Using it avoids maintaining a duplicate pipeline.

## Phased implementation plan

- **Phase 1 — Bootstrap.** `extension.toml`, license, README. _Testable:_
  `zed: install dev extension` succeeds.
- **Phase 2 — Language.** `languages/glua/config.toml` +
  `highlights.scm` + brackets / indents / outline / textobjects /
  injections. _Testable:_ syntax-highlighted `.lua` files as "GLua".
- **Phase 3 — Snippets.** `snippets/glua.json` registered in
  `extension.toml`. _Testable:_ typing `for-ipairs<Tab>` expands.
- **Phase 4 — LSP launcher.** Rust/WASM skeleton that downloads LuaLS
  via `latest_github_release` + `download_file`. _Testable:_ LuaLS
  status line shows "Downloading..." then LSP attaches.
- **Phase 5 — GLua intelligence.** `install_glua_api_addon` plus
  `language_server_workspace_configuration` that wires
  `Lua.workspace.library`, `Lua.diagnostics.globals`, `Lua.hint.*`.
  _Testable:_ `hook.Add("PlayerSpawn", ...)` gets docs on hover and
  signature help.
- **Phase 6 — Packaging.** Ensure extension builds cleanly against the
  latest `zed_extension_api`, add license + repo URL, open a PR to
  `zed-industries/extensions`.

_This repo is currently at the end of Phase 5._

## Testing the dev extension

1. Install rustup (required; Zed will not build dev extensions against a
   non-rustup Rust).
2. `zed` → command palette → `zed: install dev extension` → pick this
   directory.
3. Open any `.lua` file. The status bar should read **GLua**. If it
   reads **Lua**, drop the `file_types` override from `README.md` into
   your project settings.
4. Attach the LSP:
   ```
   -- test.lua
   hook.Add("PlayerSpawn", "test", function(ply)
       print(ply:Nick())
   end)
   ```
   - Hovering `hook.Add` should show the wiki-derived documentation.
   - Hovering `ply:Nick` should show `Player:Nick()` returning `string`.
   - Command palette → `editor: go to definition` on `Nick` should jump
     into a file under `glua-api-snippets-<version>/`.

Startup downloads (LuaLS + addon) run once per version. Subsequent
launches reuse the cached install under Zed's extension working directory.

## Open issues / blockers

1. **`.lua` collision with the Lua extension.** Zed's resolution order
   when two extensions claim the same suffix isn't stable. The README
   recommends the `file_types` override; long-term it would be nice for
   Zed to expose an extension-local priority hint.
2. **Tree-sitter doesn't know GLua operators.** `continue`, `!=`, `&&`,
   `||`, `!` highlight but may produce parse errors which degrade outline
   accuracy. Consider forking `tree-sitter-lua` to a `tree-sitter-glua`
   if this becomes painful in practice.
3. **LuaLS inlay hints don't carry the VS Code extension's realm
   emojis / flags.** The LuaCats annotations could be extended in
   `glua-api-snippets` to include the realm metadata in `@realm` tags and
   LuaLS could render them in hover; both are upstream asks, not Zed
   asks.
4. **No document color / color picker on `Color()`.** LuaLS has no
   `textDocument/documentColor`. Either contribute it upstream or build a
   minimal second LSP that only provides colors and wire it as an
   additional language server. Deferred.
5. **No in-extension UI commands.** "Find Globals" / "Localize Globals" /
   "Bytecode Heatmap" have no equivalent — Zed's extension API does not
   let you register invocable commands that touch buffers. These would
   all need to be either (a) contributed to LuaLS, (b) published as a
   standalone CLI, or (c) dropped.
6. **Icon theme.** Shipping the `.lua` / `.vmt` / `.vtf` / `.mdl` /
   `.vtx` / `.vvd` / `.phy` file icons is possible but Zed requires it
   to live in its own extension.
7. **Asset-path completions (`models/`, `materials/`, `sound/`).** No
   vanilla LSP gives you these. Would need a new sidecar LSP. Not
   attempted here.
