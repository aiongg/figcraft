//! Team library helpers: links, what is shown of them, the archive a server keeps, and the servers
//! a document's team libraries are fetched from.

use super::*;

#[test]
fn links_archives_and_junk() {
    assert!(is_server_link("figcraft://join?host=h:1&file=f&token=t") && is_server_link(" WS://h:1/?file=f&token=t"));
    assert!(!is_server_link("C:/kits/ui.figcraft") && !is_server_link("/srv/ws://x"));
    assert_eq!(shown_path("figcraft://join?host=h:1&file=f&token=secret&x=1"), "figcraft://join?host=h:1&file=f&token=…&x=1");
    assert_eq!(shown_path("ws://h:1/?file=f&token=secret"), "ws://h:1/?file=f&token=…");
    assert_eq!(shown_path("/kits/token=ui.figcraft"), "/kits/token=ui.figcraft", "files are shown as they are");

    let mut s = Session::new();
    let f = s.execute("frame.create", &serde_json::json!({"x": 0, "y": 0, "width": 10, "height": 10, "name": "Chip"})).unwrap();
    s.execute("component.create", &serde_json::json!({"ids": [f["id"]]})).unwrap();
    s.execute("library.publish", &serde_json::json!({})).unwrap();
    let lib = s.doc().unwrap().published.clone().unwrap();
    let back = read_archive(&archive(&lib).unwrap()).unwrap();
    assert_eq!(back.manifest, lib.manifest);
    assert!(read_archive(b"not a zip").is_err());
    let plain = figcraft_format::save_bytes(&Document::new("x"), None).unwrap();
    assert!(read_archive(&plain).unwrap_err().contains("without a library"));
    // Without the app's networking, a team library is a clear error.
    assert!(
        s.execute("library.add", &serde_json::json!({"path": "ws://127.0.0.1:9/?file=f&token=t"})).unwrap_err().to_string().contains("desktop app")
    );
}

/// A link that records the team libraries asked for in the background.
struct Asked(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl crate::live::LiveLink for Asked {
    fn host(&mut self, _: u64, _: crate::live::HostOptions) -> std::result::Result<crate::live::SessionInfo, String> {
        Err("no".into())
    }
    fn join(&mut self, _: u64, _: &str, _: &str) -> std::result::Result<(), String> {
        Err("no".into())
    }
    fn library_background(&mut self, jobs: Vec<(String, LibraryReq)>) {
        self.0.lock().unwrap().extend(jobs.into_iter().map(|(link, _)| link));
    }
    fn leave(&mut self, _: u64) {}
    fn presence(&mut self, _: u64, _: &crate::live::Presence) {}
    fn send(&mut self, _: u64, _: figcraft_collab::message::ClientMsg) {}
    fn poll(&mut self) -> Vec<(u64, crate::live::Event)> {
        Vec::new()
    }
}

#[test]
fn a_documents_team_libraries_are_fetched_only_from_servers_the_user_allowed() {
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut s = Session::new();
    s.live.install(Box::new(Asked(asked.clone())));
    let (a, b) = ("wss://Libs.Example:8443/file/1?token=t", "figcraft://join?host=10.0.0.2:7000&file=f&token=t");
    for (id, path) in [("a", a), ("b", b), ("c", "wss://libs.example:8443/file/2")] {
        s.doc_mut().unwrap().libraries_mut().enabled.push(figcraft_doc::LibraryLink { id: id.into(), name: id.into(), path: path.into(), version: 1 });
    }
    assert_eq!(s.library_hosts_to_confirm(), ["10.0.0.2:7000", "libs.example:8443"]);
    s.refresh_server_libraries();
    assert!(asked.lock().unwrap().is_empty(), "nothing is fetched before the user answers");
    s.libraries.allow_host("libs.example:8443", true);
    s.libraries.allow_host("10.0.0.2:7000", false);
    assert!(s.library_hosts_to_confirm().is_empty());
    s.refresh_server_libraries();
    assert_eq!(*asked.lock().unwrap(), [a, "wss://libs.example:8443/file/2"]);
    let e = s.fetch_library(b, true).unwrap_err().to_string();
    assert!(e.contains("10.0.0.2:7000"), "{e}");
    assert!(s.libraries.cached(b).is_none());
}
