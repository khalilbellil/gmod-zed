; TODO/FIXME/NOTE/WARN/HACK highlighting inside comments.
((comment) @injection.content
 (#set! injection.language "comment"))

; LuaJIT FFI: `ffi.cdef[[ ... ]]` injects C.
((function_call
    name: [
        (identifier) @_cdef_identifier
        (_
            _
            (identifier) @_cdef_identifier)
    ]
    arguments: (arguments
        (string
            content: _ @injection.content
            (#set! injection.language "c"))))
 (#eq? @_cdef_identifier "cdef"))
