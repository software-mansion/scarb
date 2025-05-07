use scarb_test_support::command::Scarb;

static BUILTIN_COMMANDS: &[&str] = &[
    "add",
    "remove",
    "build",
    "expand",
    "cache",
    "check",
    "clean",
    "commands",
    "fetch",
    "fmt",
    "init",
    "manifest-path",
    "metadata",
    "new",
    "package",
    "proc-macro-server",
    "publish",
    "lint",
    "run",
    "test",
    "update",
];

// This list is not exhaustive and only covers the basic options, as some options are handled differently depending on the shell.
static GLOBAL_OPTIONS: &[&str] = &[
    "--manifest-path",
    "--verbosity",
    "--global-cache-dir",
    "--global-config-dir",
    "--target-dir",
];

static EXTERNAL_COMMANDS: &[&str] = &[
    "cairo-language-server",
    "cairo-run",
    "cairo-test",
    "completions",
    "doc",
    "execute",
    "mdbook",
    "prove",
    "verify",
];

#[test]
fn generates_completions_bash() {
    let cmd = Scarb::quick_snapbox()
        .arg("completions")
        .arg("bash")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| l.trim() == "_scarb() {"));

    for cmd in BUILTIN_COMMANDS {
        let line = format!("scarb,{}{}", cmd, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("{}{}", opt, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("scarb,{}{}", ext, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
}

#[test]
fn generates_completions_zsh() {
    let cmd = Scarb::quick_snapbox()
        .arg("completions")
        .arg("zsh")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| l.trim() == "#compdef scarb"));

    for cmd in BUILTIN_COMMANDS {
        let line = format!("({})", cmd);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("'{}=", opt);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("({})", ext);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
}

#[test]
fn generates_completions_powershell() {
    let cmd = Scarb::quick_snapbox()
        .arg("completions")
        .arg("powershell")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| {
        l.contains("Register-ArgumentCompleter -Native -CommandName 'scarb' -ScriptBlock {")
    }));

    for cmd in BUILTIN_COMMANDS {
        let line = format!("[CompletionResult]::new('{}'", cmd);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("[CompletionResult]::new('{}'", opt);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("[CompletionResult]::new('{}'", ext);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
}

#[test]
fn generates_completions_fish() {
    let cmd = Scarb::quick_snapbox()
        .arg("completions")
        .arg("fish")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| {
        l.trim()
            .starts_with("function __fish_scarb_global_optspecs")
    }));

    for cmd in BUILTIN_COMMANDS {
        let line = format!(
            "complete -c scarb -n \"__fish_scarb_needs_command\" -f -a \"{}\"",
            cmd
        );
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!(
            "complete -c scarb -n \"__fish_scarb_needs_command\" -l {}",
            opt.trim_start_matches('-')
        );
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!(
            "complete -c scarb -n \"__fish_scarb_needs_command\" -f -a \"{}\"",
            ext
        );
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
}

#[test]
fn generates_completions_elvish() {
    let cmd = Scarb::quick_snapbox()
        .arg("completions")
        .arg("elvish")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(
        lines
            .iter()
            .any(|l| l.contains("set edit:completion:arg-completer[scarb] = {|@words|"))
    );

    for cmd in BUILTIN_COMMANDS {
        let line = format!("cand {} ", cmd);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("cand {} ", opt);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("cand {} ", ext);
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command:f {ext}"
        );
    }
}
