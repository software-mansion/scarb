use indoc::indoc;
use scarb_test_support::command::Scarb;

#[test]
fn generates_completions_bash() {
    Scarb::quick_snapbox()
        .arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            _scarb() {
            [..]local i cur prev opts cmd
            [..]COMPREPLY=()
            ...
        "#});
}

#[test]
fn generates_completions_zsh() {
    Scarb::quick_snapbox()
        .arg("completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            #compdef scarb

            autoload -U is-at-least

            _scarb() {
            ...
        "#});
}

#[test]
fn generates_completions_powershell() {
    Scarb::quick_snapbox()
        .arg("completions")
        .arg("powershell")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"

            using namespace System.Management.Automation
            using namespace System.Management.Automation.Language

            Register-ArgumentCompleter -Native -CommandName 'scarb' -ScriptBlock {
            [..]param($wordToComplete, $commandAst, $cursorPosition)
            ...
        "#});
}

#[test]
fn generates_completions_fish() {
    Scarb::quick_snapbox()
        .arg("completions")
        .arg("fish")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            # Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
            function __fish_scarb_global_optspecs
            [..]string join [..]
            end
            ...
        "#});
}

#[test]
fn generates_completions_elvish() {
    Scarb::quick_snapbox()
        .arg("completions")
        .arg("elvish")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"

            use builtin;
            use str;

            set edit:completion:arg-completer[scarb] = {|@words|
            ...
        "#});
}
