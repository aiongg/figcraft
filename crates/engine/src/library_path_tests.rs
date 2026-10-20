//! A library link is stored in the document, so it is followed only on this computer
//! ([`crate::library_server`]): a path on another server would make opening someone else's file
//! contact that server, and on Windows send the credentials of whoever opened it.

use crate::Session;

#[test]
fn a_library_is_opened_only_from_this_computer() {
    let mut s = Session::new();
    for path in [
        r"\\attacker.example\share\lib.figcraft",
        "//attacker.example/share/lib.figcraft",
        "file://attacker.example/share/lib.figcraft",
        r"\\?\UNC\attacker.example\share\lib.figcraft",
        r"\\.\pipe\lib",
        "https://attacker.example/lib.figcraft",
    ] {
        let e = s.fetch_library(path, false).expect_err(&format!("{path} must be refused"));
        assert!(e.to_string().contains("only from this computer"), "{path}: {e}");
    }
}

#[test]
fn a_local_library_path_is_still_followed() {
    let mut s = Session::new();
    // Refused for being absent, not for where it is: the gate let it through.
    let missing = std::env::temp_dir().join("figcraft-no-such-library.figcraft");
    let e = s.fetch_library(&missing.to_string_lossy(), false).expect_err("the file does not exist");
    assert!(!e.to_string().contains("only from this computer"), "a local path must not be refused as remote: {e}");
}

#[test]
fn a_relative_library_link_is_taken_from_the_document_folder() {
    let d = std::env::temp_dir().join(format!("figcraft-relative-library-{}", std::process::id()));
    std::fs::create_dir_all(d.join("libs")).unwrap();
    let ok = |s: &mut Session, cmd: &str, p: serde_json::Value| s.execute(cmd, &p).unwrap_or_else(|e| panic!("{cmd}: {e}"));
    let mut lib = Session::new();
    let f = ok(&mut lib, "frame.create", serde_json::json!({"x": 0, "y": 0, "width": 10, "height": 10, "name": "Button"}));
    ok(&mut lib, "component.create", serde_json::json!({"ids": [f["id"]]}));
    ok(&mut lib, "library.publish", serde_json::json!({}));
    ok(&mut lib, "file.saveAs", serde_json::json!({"path": d.join("libs").join("kit.figcraft")}));

    let mut unsaved = Session::new();
    let e = unsaved.fetch_library("libs/kit.figcraft", false).expect_err("an unsaved document has no folder");
    assert!(e.to_string().contains("not opened"), "{e}");
    let mut s = Session::new();
    ok(&mut s, "file.saveAs", serde_json::json!({"path": d.join("doc.figcraft")}));
    assert!(s.fetch_library("libs/kit.figcraft", false).is_ok());
    assert!(s.libraries.cached("libs/kit.figcraft").is_some_and(|r| r.is_ok()), "remembered under the link");
    let _ = std::fs::remove_dir_all(&d);
}
