use indoc::indoc;
use scarb_test_support::command::Scarb;
use snapbox::Data;

static BUILTIN_COMMANDS: &[&str] = &[
    "add",
    "remove",
    "build",
    "expand",
    "cache",
    "check",
    "clean",
    "completions",
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

// TODO(#1915): Enable `prove` and `verify` checks once stwo is stable.
// Currently, `prove` and `verify` are not built when running tests on CI, so we cannot check for them.
static EXTERNAL_COMMANDS: &[&str] = &[
    "cairo-language-server",
    "cairo-test",
    "doc",
    "execute",
    "mdbook",
    // "prove",
    // "verify",
];

#[test]
#[ignore]
fn generates_completions_bash() {
    let cmd = Scarb::quick_command()
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
    for ext in EXTERNAL_COMMANDS {
        let line = format!("scarb,{}{}", ext, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("{}{}", opt, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
}

#[test]
#[ignore]
fn generates_completions_zsh() {
    let cmd = Scarb::quick_command()
        .arg("completions")
        .arg("zsh")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| l.trim() == "#compdef scarb"));

    for cmd in BUILTIN_COMMANDS {
        let line = format!("({cmd})");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("({ext})");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("'{opt}=");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
}

#[test]
#[ignore]
fn generates_completions_powershell() {
    let cmd = Scarb::quick_command()
        .arg("completions")
        .arg("powershell")
        .assert()
        .success();
    let output = cmd.get_output().stdout.clone();
    let stdout = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();

    assert!(lines.iter().any(|l| {
        l.starts_with("Register-ArgumentCompleter -Native -CommandName 'scarb' -ScriptBlock {")
    }));

    for cmd in BUILTIN_COMMANDS {
        let line = format!("[CompletionResult]::new('{cmd}'");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("[CompletionResult]::new('{ext}'");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("[CompletionResult]::new('{opt}'");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
}

#[test]
#[ignore]
fn generates_completions_fish() {
    let cmd = Scarb::quick_command()
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
        let line = format!("complete -c scarb -n \"__fish_scarb_needs_command\" -f -a \"{cmd}\"");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("complete -c scarb -n \"__fish_scarb_needs_command\" -f -a \"{ext}\"");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
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
}

#[test]
#[ignore]
fn generates_completions_elvish() {
    let cmd = Scarb::quick_command()
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
            .any(|l| l.starts_with("set edit:completion:arg-completer[scarb] = {|@words|"))
    );

    for cmd in BUILTIN_COMMANDS {
        let line = format!("cand {cmd} ");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing command: {cmd}"
        );
    }
    for ext in EXTERNAL_COMMANDS {
        let line = format!("cand {ext} ");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command:f {ext}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("cand {opt} ");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
}

#[test]
#[ignore]
fn generates_completions_without_arg() {
    let cmd = Scarb::quick_command()
        .arg("completions")
        .env("SHELL", "bash")
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
    for ext in EXTERNAL_COMMANDS {
        let line = format!("scarb,{}{}", ext, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing external command: {ext}"
        );
    }
    for opt in GLOBAL_OPTIONS {
        let line = format!("{}{}", opt, ")");
        assert!(
            lines.iter().any(|l| l.trim().starts_with(&line)),
            "Missing option: {opt}"
        );
    }
}

// Disabled due to `clap_complete::Shell::from_env` defaulting to `PowerShell` on Windows
#[cfg(not(windows))]
#[test]
fn fails_without_arg_and_empty_env() {
    Scarb::quick_command()
        .arg("completions")
        .env("SHELL", "")
        .assert()
        .failure()
        .stdout_eq(
            Data::from(indoc!(
                r#"
            error: could not automatically determine shell to generate completions for
            help: specify the shell explicitly: `scarb completions <shell>`
            for the list of supported shells, run `scarb completions --help`
        "#
            ))
            .raw(),
        );
}
