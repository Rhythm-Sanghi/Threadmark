use std::{sync::mpsc, time::Duration};
use threadmark_core::watch::WatchHandle;
#[test]
fn native_watcher_reparses_changed_files_reuses_others_and_handles_rename_delete() {
    let root = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("work.ts"), "export function work(){}").unwrap();
    std::fs::write(root.path().join("stable.ts"), "export function stable(){}").unwrap();
    let (sender, receiver) = mpsc::channel();
    let watch = WatchHandle::start(root.path(), data.path().to_owned(), move |snapshot| {
        sender.send(snapshot).unwrap();
    })
    .unwrap();
    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap()
            .progress
            .parsed_files,
        2
    );
    std::fs::write(root.path().join("work.ts"), "export function changed(){}").unwrap();
    let mut changed = false;
    for _ in 0..4 {
        let snapshot = receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        if snapshot.nodes.iter().any(|node| node.name == "changed") {
            assert_eq!(snapshot.progress.parsed_files, 1);
            assert_eq!(snapshot.progress.reused_files, 1);
            changed = true;
            break;
        }
    }
    assert!(changed);
    std::fs::rename(root.path().join("work.ts"), root.path().join("renamed.ts")).unwrap();
    std::fs::remove_file(root.path().join("stable.ts")).unwrap();
    let mut renamed = false;
    for _ in 0..4 {
        let snapshot = receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        if snapshot.nodes.iter().any(|node| node.path == "renamed.ts")
            && !snapshot.nodes.iter().any(|node| node.path == "stable.ts")
        {
            renamed = true;
            break;
        }
    }
    assert!(renamed);
    drop(watch);
}
