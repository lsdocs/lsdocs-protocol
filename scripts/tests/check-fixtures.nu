# Tests for scripts/check-fixtures.nu. Run with `nur test scripts`.

use std/assert
use ../check-fixtures.nu [check normalize]

# Writes a fixture pair into `dir`: `value` as MessagePack and `json` as JSON.
def pair [dir: path, name: string, value: any, json: any] {
    $value | to msgpack | save --raw ($dir | path join $"($name).msgpack")
    $json | to json | save ($dir | path join $"($name).json")
}

def with-temp-dir [body: closure] {
    let dir = mktemp --directory
    try { do $body $dir } catch {|e| rm --recursive --force $dir; error make $e.raw }
    rm --recursive --force $dir
}

def "test matching map passes" [] {
    with-temp-dir {|dir|
        pair $dir ok {name: "lsdocs", version: "0.1.0"} {name: "lsdocs", version: "0.1.0"}
        assert equal (check $dir) []
    }
}

def "test positional array is caught" [] {
    with-temp-dir {|dir|
        pair $dir info ["lsdocs" "0.1.0"] {name: "lsdocs", version: "0.1.0"}
        let errors = check $dir
        assert equal ($errors | length) 1
        assert str contains $errors.0 "info.msgpack: decoded MessagePack differs from JSON"
    }
}

def "test binary matches json byte list" [] {
    with-temp-dir {|dir|
        pair $dir render {output: 0x[3c 70 3e], extension: "html"} {output: [60 112 62], extension: "html"}
        assert equal (check $dir) []
    }
}

def "test missing twin is caught" [] {
    with-temp-dir {|dir|
        {a: 1} | to msgpack | save --raw ($dir | path join "lonely.msgpack")
        assert equal (check $dir) ["lonely.msgpack: missing JSON twin"]
    }
}

def "test normalize recurses" [] {
    let value = {outer: [{b: 0x[01 02]}]} | normalize
    assert equal $value {outer: [{b: [1 2]}]}
}

# Runs every test in this file.
export def main [] {
    test matching map passes
    test positional array is caught
    test binary matches json byte list
    test missing twin is caught
    test normalize recurses
    print "check-fixtures tests: 5 passed"
}
