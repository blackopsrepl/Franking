use super::FilePickerState;

#[test]
fn lists_directories_before_files() {
    let root = std::env::temp_dir().join(format!(
        "sfmail-picker-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    std::fs::write(root.join(".hidden"), b"h").unwrap();

    let state = FilePickerState::open(Some(root.clone()));
    let names: Vec<String> = state
        .entries
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(
        names,
        vec!["sub", "a.txt"],
        "directories first, dotfiles hidden"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn descends_and_moves_back_up() {
    let root = std::env::temp_dir().join(format!(
        "sfmail-picker-nav-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("sub").join("file.txt"), b"x").unwrap();

    let mut state = FilePickerState::open(Some(root.clone()));
    // The only entry is the directory, so Enter descends instead of selecting.
    assert!(state.activate().is_none());
    assert_eq!(state.dir, root.join("sub"));

    let picked = state.activate().expect("file picked");
    assert_eq!(picked, root.join("sub").join("file.txt"));

    state.parent_directory();
    assert_eq!(state.dir, root);

    let _ = std::fs::remove_dir_all(&root);
}
