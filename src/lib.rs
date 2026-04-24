//! gmod-zed: Garry's Mod GLua extension for Zed.
//!
//! Boots the LuaLS language server
//! (<https://github.com/LuaLS/lua-language-server>) and points it at the
//! community-maintained `glua-api-snippets` addon
//! (<https://github.com/luttje/glua-api-snippets>) so completions, hovers,
//! signatures, and go-to-definition line up with the Garry's Mod wiki.
//!
//! This is the Zed-native analogue of the VS Code extension's
//! `WikiProvider` + `CompletionProvider` + `HoverProvider` + `SignatureProvider`
//! + `DefinitionProvider` stack. All of that work is delegated to LuaLS now;
//! the only job of this crate is to install LuaLS + the GLua addon and stream
//! the right workspace configuration to LuaLS.

use std::fs;
use zed_extension_api::{
    self as zed, lsp::CompletionKind, serde_json, settings::LspSettings, CodeLabel, CodeLabelSpan,
    LanguageServerId, Result,
};

const LUALS_REPO: &str = "LuaLS/lua-language-server";
const GLUA_API_REPO: &str = "luttje/glua-api-snippets";
const GLUA_API_DIR_PREFIX: &str = "glua-api-snippets-";

struct GmodExtension {
    cached_luals_path: Option<String>,
    cached_glua_api_dir: Option<String>,
}

impl GmodExtension {
    fn resolve_luals_binary(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<(String, Vec<String>)> {
        // `CommandSettings` is not `Clone` in zed_extension_api 0.7, so we
        // consume the settings once and extract the leaf fields (which are
        // `Clone`) by move/reference.
        let binary = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|s| s.binary);
        let args = binary
            .as_ref()
            .and_then(|b| b.arguments.clone())
            .unwrap_or_default();
        let explicit_path = binary.and_then(|b| b.path);

        if let Some(path) = explicit_path {
            return Ok((path, args));
        }

        if let Some(path) = worktree.which("lua-language-server") {
            return Ok((path, args));
        }

        let managed = self.install_luals(language_server_id)?;
        Ok((managed, args))
    }

    fn install_luals(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = &self.cached_luals_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            LUALS_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let asset_name = format!(
            "lua-language-server-{version}-{os}-{arch}.{ext}",
            version = release.version,
            os = match platform {
                zed::Os::Mac => "darwin",
                zed::Os::Linux => "linux",
                zed::Os::Windows => "win32",
            },
            arch = match arch {
                zed::Architecture::Aarch64 => "arm64",
                zed::Architecture::X8664 => "x64",
                zed::Architecture::X86 => return Err("unsupported platform: x86".into()),
            },
            ext = match platform {
                zed::Os::Mac | zed::Os::Linux => "tar.gz",
                zed::Os::Windows => "zip",
            },
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no LuaLS asset found matching {asset_name:?}"))?;

        let version_dir = format!("lua-language-server-{}", release.version);
        let binary_path = format!(
            "{version_dir}/bin/lua-language-server{ext}",
            ext = match platform {
                zed::Os::Mac | zed::Os::Linux => "",
                zed::Os::Windows => ".exe",
            }
        );

        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                match platform {
                    zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
                    zed::Os::Windows => zed::DownloadedFileType::Zip,
                },
            )
            .map_err(|e| format!("failed to download LuaLS: {e}"))?;

            self.prune_stale_dirs(Some(&version_dir), None)?;
        }

        self.cached_luals_path = Some(binary_path.clone());
        Ok(binary_path)
    }

    fn install_glua_api_addon(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(dir) = &self.cached_glua_api_dir {
            if fs::metadata(dir).is_ok_and(|stat| stat.is_dir()) {
                return Ok(dir.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GLUA_API_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        // Releases attach a `<tag>.lua.zip` (workspace-ready annotations) and
        // a `<tag>.json.zip` (raw wiki JSON). We want the LuaCats-annotated
        // `.lua` files — LuaLS reads them directly via `workspace.library`.
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".lua.zip"))
            .ok_or_else(|| "no .lua.zip asset in glua-api-snippets release".to_string())?;

        let version_dir = format!("{GLUA_API_DIR_PREFIX}{}", release.version);
        if !fs::metadata(&version_dir).is_ok_and(|stat| stat.is_dir()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            eprintln!(
                "gmod-zed: downloading glua-api-snippets {} from {}",
                release.version, asset.download_url
            );

            zed::download_file(
                &asset.download_url,
                &version_dir,
                zed::DownloadedFileType::Zip,
            )
            .map_err(|e| format!("failed to download glua-api-snippets: {e}"))?;

            self.prune_stale_dirs(None, Some(&version_dir))?;
        }

        // LuaLS resolves `workspace.library` entries that are not absolute
        // against the user's workspace root — not our extension dir — so we
        // must hand it an absolute path. Canonicalize the freshly-downloaded
        // directory so LuaLS can load the annotations regardless of where the
        // user's project lives.
        let absolute = fs::canonicalize(&version_dir)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|e| {
                eprintln!(
                    "gmod-zed: canonicalize({version_dir:?}) failed ({e}); \
                     falling back to relative path"
                );
                version_dir.clone()
            });

        eprintln!("gmod-zed: glua-api-snippets library path = {absolute}");

        self.cached_glua_api_dir = Some(absolute.clone());
        Ok(absolute)
    }

    /// Remove previous LuaLS / GLua-addon installs so we don't balloon disk
    /// usage on each upgrade.
    fn prune_stale_dirs(&self, keep_luals: Option<&str>, keep_addon: Option<&str>) -> Result<()> {
        let entries = fs::read_dir(".").map_err(|e| format!("failed to list work dir: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read dir entry: {e}"))?;
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else {
                continue;
            };

            let is_luals = name_str.starts_with("lua-language-server-");
            let is_addon = name_str.starts_with(GLUA_API_DIR_PREFIX);
            let keep = (is_luals && Some(name_str) == keep_luals)
                || (is_addon && Some(name_str) == keep_addon);

            if (is_luals || is_addon) && !keep {
                fs::remove_dir_all(entry.path()).ok();
            }
        }
        Ok(())
    }
}

impl zed::Extension for GmodExtension {
    fn new() -> Self {
        Self {
            cached_luals_path: None,
            cached_glua_api_dir: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let (command, args) = self.resolve_luals_binary(language_server_id, worktree)?;
        Ok(zed::Command {
            command,
            args,
            env: vec![],
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|s| s.initialization_options.clone()))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        let user_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|s| s.settings.clone());

        // Best-effort: we still start LuaLS even if the GLua addon download
        // fails — users just won't get wiki-sourced completions.
        let glua_library = match self.install_glua_api_addon(language_server_id) {
            Ok(path) => vec![path],
            Err(err) => {
                eprintln!("gmod-zed: couldn't install glua-api-snippets: {err}");
                Vec::new()
            }
        };

        // Sensible defaults for a GMod workspace. Users can override any of
        // these via `lsp.LuaLS (GLua).settings` in their Zed settings.
        let mut config = serde_json::json!({
            "Lua": {
                "runtime": {
                    "version": "Lua 5.1",
                    "special": {
                        "AddCSLuaFile": "require",
                        "include": "require"
                    }
                },
                "diagnostics": {
                    "globals": [
                        "_G", "_VERSION",
                        "CLIENT", "SERVER", "MENU_DLL", "MENU",
                        "SysTime", "RealTime", "CurTime", "UnPredictedCurTime",
                        "IsValid", "isnumber", "isstring", "isbool", "istable",
                        "isvector", "isangle", "isfunction", "isentity", "ispanel", "ismatrix",
                        "Color", "Vector", "Angle", "Matrix",
                        "Entity", "Player", "Material", "Sound", "CreateSound",
                        "LocalPlayer", "ScrW", "ScrH", "ScreenScale",
                        "AddCSLuaFile", "include", "Msg", "MsgN", "MsgC", "print", "PrintTable",
                        "RunConsoleCommand", "CreateConVar", "CreateClientConVar",
                        "hook", "net", "util", "ents", "game", "timer", "concommand",
                        "surface", "draw", "gui", "input", "vgui", "cam", "render",
                        "physics", "scripted_ents", "weapons", "list", "team",
                        "properties", "spawnmenu", "language", "cookie", "cvars",
                        "http", "navmesh", "ai", "ai_schedule", "ai_task",
                        "NULL",
                        "FCVAR_ARCHIVE", "FCVAR_NOTIFY", "FCVAR_REPLICATED",
                        "FCVAR_USERINFO", "FCVAR_PRINTABLEONLY",
                        "FCVAR_SERVER_CAN_EXECUTE", "FCVAR_CHEAT"
                    ],
                    "disable": ["lowercase-global"]
                },
                "workspace": {
                    "checkThirdParty": false,
                    "library": glua_library
                },
                "hint": {
                    "enable": true,
                    "arrayIndex": "Disable",
                    "paramName": "All",
                    "paramType": true,
                    "setType": true
                },
                "telemetry": { "enable": false },
                "format": { "enable": false }
            }
        });

        if let Some(overrides) = user_settings {
            deep_merge(&mut config, &overrides);
        }

        if let Some(lib) = config
            .get("Lua")
            .and_then(|l| l.get("workspace"))
            .and_then(|w| w.get("library"))
        {
            eprintln!("gmod-zed: sending Lua.workspace.library = {lib}");
        }

        Ok(Some(config))
    }

    // Render `foo(arg1, arg2)` completions so the filter range only matches
    // the function name, matching the Lua extension's behaviour.
    fn label_for_completion(
        &self,
        _language_server_id: &LanguageServerId,
        completion: zed::lsp::Completion,
    ) -> Option<CodeLabel> {
        match completion.kind? {
            CompletionKind::Method | CompletionKind::Function => {
                let name_len = completion.label.find('(').unwrap_or(completion.label.len());
                Some(CodeLabel {
                    spans: vec![CodeLabelSpan::code_range(0..completion.label.len())],
                    filter_range: (0..name_len).into(),
                    code: completion.label,
                })
            }
            CompletionKind::Field => Some(CodeLabel {
                spans: vec![CodeLabelSpan::literal(
                    completion.label.clone(),
                    Some("property".into()),
                )],
                filter_range: (0..completion.label.len()).into(),
                code: Default::default(),
            }),
            _ => None,
        }
    }

    fn label_for_symbol(
        &self,
        _language_server_id: &LanguageServerId,
        symbol: zed::lsp::Symbol,
    ) -> Option<CodeLabel> {
        let prefix = "local a = ";
        let suffix = match symbol.kind {
            zed::lsp::SymbolKind::Method => "()",
            _ => "",
        };
        let code = format!("{prefix}{}{suffix}", symbol.name);
        Some(CodeLabel {
            spans: vec![CodeLabelSpan::code_range(
                prefix.len()..code.len() - suffix.len(),
            )],
            filter_range: (0..symbol.name.len()).into(),
            code,
        })
    }
}

/// Recursively merges `overlay` into `base`. Object keys are unioned; other
/// values are replaced wholesale. Mirrors the behaviour most LSP configs
/// expect when users override a nested setting.
fn deep_merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (k, v) in o {
                deep_merge(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, o) => *b = o.clone(),
    }
}

zed::register_extension!(GmodExtension);
