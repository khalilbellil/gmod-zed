; Based on zed-extensions/lua (Apache-2.0) with Garry's Mod additions.

; Keywords
[
    "do"
    "else"
    "elseif"
    "end"
    "for"
    "function"
    "goto"
    "if"
    "in"
    "local"
    "repeat"
    "return"
    "then"
    "until"
    "while"
    (break_statement)
] @keyword

; GLua adds `continue` as a keyword (parsed as identifier by tree-sitter-lua).
((identifier) @keyword
 (#eq? @keyword "continue"))

; Operators
[
    "and"
    "not"
    "or"
] @keyword.operator

[
    "+"
    "-"
    "*"
    "/"
    "%"
    "^"
    "#"
    "=="
    "~="
    "<="
    ">="
    "<"
    ">"
    "="
    "&"
    "~"
    "|"
    "<<"
    ">>"
    "//"
    ".."
] @operator

; Punctuations
[
    ";"
    ":"
    ","
    "."
] @punctuation.delimiter

; Brackets
[
    "("
    ")"
    "["
    "]"
    "{"
    "}"
] @punctuation.bracket

; Variables
(identifier) @variable

((identifier) @variable.special
 (#eq? @variable.special "self"))

(variable_list
    attribute: (attribute
        ([
            "<"
            ">"
        ] @punctuation.bracket
        (identifier) @attribute)))

; Constants (SCREAMING_CASE)
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_0-9]*$"))

(vararg_expression) @constant

(nil) @constant.builtin

[
    (false)
    (true)
] @boolean

; Garry's Mod built-in constants.
((identifier) @constant.builtin
 (#any-of? @constant.builtin
    "CLIENT" "SERVER" "MENU_DLL" "MENU" "NULL" "_G" "_VERSION"
    "FCVAR_ARCHIVE" "FCVAR_NOTIFY" "FCVAR_REPLICATED" "FCVAR_USERINFO"
    "FCVAR_PRINTABLEONLY" "FCVAR_SERVER_CAN_EXECUTE" "FCVAR_CHEAT"
    "FCVAR_UNREGISTERED" "FCVAR_LUA_SERVER" "FCVAR_LUA_CLIENT"))

; Tables
(field
    name: (identifier) @property)

(dot_index_expression
    field: (identifier) @property)

(table_constructor
    [
        "{"
        "}"
    ] @constructor)

; Functions
(parameters
    (identifier) @parameter)

(function_call
    name: [
        (identifier) @function
        (dot_index_expression
            field: (identifier) @function)
    ])

(function_declaration
    name: [
        (identifier) @function.definition
        (dot_index_expression
            field: (identifier) @function.definition)
    ])

(method_index_expression
    method: (identifier) @function.method)

; Lua 5.1 built-ins (Garry's Mod uses LuaJIT/Lua 5.1).
(function_call
    (identifier) @function.builtin
    (#any-of? @function.builtin
        "assert" "collectgarbage" "dofile" "error" "getfenv" "getmetatable" "ipairs" "load" "loadfile"
        "loadstring" "module" "next" "pairs" "pcall" "print" "rawequal" "rawget" "rawset" "require"
        "select" "setfenv" "setmetatable" "tonumber" "tostring" "type" "unpack" "xpcall"))

; Common Garry's Mod globals and constructors.
(function_call
    (identifier) @function.builtin
    (#any-of? @function.builtin
        "Color" "Vector" "Angle" "Matrix" "Entity" "Player" "Material" "Sound"
        "IsValid" "CreateClientConVar" "CreateConVar" "AddCSLuaFile" "include"
        "Msg" "MsgN" "MsgC" "PrintTable" "ScreenScale" "ScrW" "ScrH" "LocalPlayer"
        "RunConsoleCommand" "ProtectedCall" "SafeRemoveEntity" "timer" "hook" "net"
        "util" "ents" "game" "concommand" "surface" "draw" "gui" "input" "vgui"))

; Garry's Mod top-level namespace identifiers used as tables (hook.Add, net.Start, etc.).
((identifier) @type.builtin
 (#any-of? @type.builtin
    "hook" "net" "util" "ents" "player" "game" "timer" "concommand"
    "surface" "draw" "gui" "input" "vgui" "file" "string" "table" "math"
    "os" "io" "debug" "coroutine" "bit" "cam" "render" "sound" "physics"
    "resource" "scripted_ents" "weapons" "list" "team" "umsg" "usermessage"
    "properties" "spawnmenu" "language" "cookie" "cvars" "system" "steamworks"
    "serverlist" "notification" "matproxy" "navmesh" "http" "ai" "ai_schedule"
    "ai_task"))

; Strings and escapes
(comment) @comment

(hash_bang_line) @preproc

(number) @number

(string) @string

(escape_sequence) @string.escape
