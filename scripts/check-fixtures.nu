# Decodes every golden MessagePack fixture with nushell's own decoder and
# checks it equals its JSON twin. This is a second, non-serde reader of the
# wire format: a struct accidentally encoded as a positional array, or an enum
# in an unexpected shape, shows up as a mismatch.
#
# Run with `nur test fixtures`. Regenerate fixtures with `nur gen`.

# Converts nushell's decoded MessagePack into the shape JSON gives: `bin`
# values become lists of byte values (how serde writes bytes to JSON).
export def normalize []: any -> any {
    let value = $in
    let type = $value | describe | str replace --regex '<.*' ''
    match $type {
        "binary" => ($value | chunks 1 | each {|b| $b | into int })
        "record" => ($value | items {|k, v| {$k: ($v | normalize)} } | into record)
        "list" | "table" => ($value | each {|v| $v | normalize })
        _ => $value
    }
}

# Absolute paths of the `.msgpack` files under an absolute `root`, sorted.
# Glob patterns need `/`: a `\` would be read as an escape.
def fixture-files [root: path]: nothing -> list<path> {
    glob $"($root | str replace --all '\' '/')/**/*.msgpack" | sort
}

# Returns a list of mismatch messages for the fixtures under `dir`.
export def check [dir: path]: nothing -> list<string> {
    let root = $dir | path expand
    fixture-files $root | each {|file|
        let twin = $file | str replace --regex '\.msgpack$' '.json'
        let name = $file | path relative-to $root | str replace --all '\' '/'
        if not ($twin | path exists) {
            $"($name): missing JSON twin"
        } else {
            let decoded = open --raw $file | from msgpack | normalize
            let expected = open --raw $twin | from json
            if $decoded == $expected {
                null
            } else {
                $"($name): decoded MessagePack differs from JSON\n  msgpack: ($decoded | to json --raw)\n  json:    ($expected | to json --raw)"
            }
        }
    } | compact
}

# Fails if any fixture's MessagePack does not match its JSON twin.
export def main [dir: path = "fixtures"]: nothing -> nothing {
    let count = fixture-files ($dir | path expand) | length
    if $count == 0 {
        error make --unspanned {msg: $"no fixtures found under ($dir); run `nur gen`"}
    }
    let errors = check $dir
    if ($errors | is-empty) {
        print $"check-fixtures: ($count) fixtures match their JSON twins"
    } else {
        for e in $errors { print -e $"error: ($e)" }
        error make --unspanned {msg: $"check-fixtures: ($errors | length) mismatch\(es\)"}
    }
}
