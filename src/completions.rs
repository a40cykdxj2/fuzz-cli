//! Shell completion snippets, printed with `fz --completions <shell>`.

const BASH: &str = r#"_fz() {
    local current
    current="${COMP_WORDS[COMP_CWORD]}"
    COMPREPLY=($(compgen -d -- "$current"))
}
complete -o filenames -F _fz fz
"#;

const ZSH: &str = r#"#compdef fz

_fz() {
    _arguments \
        '1:directory:_files -/' \
        '(-p --preview-lines)'{-p,--preview-lines}'[lines shown in the preview pane]:lines:' \
        '(-d --max-depth)'{-d,--max-depth}'[directory depth limit]:depth:' \
        '--no-preview[skip the preview pane]'
}
"#;

const FISH: &str = r#"complete -c fz -x -a "(__fish_complete_directories)"
complete -c fz -s p -l preview-lines -x -d "Lines shown in the preview pane"
complete -c fz -s d -l max-depth -x -d "Directory depth limit"
complete -c fz -l no-preview -d "Skip the preview pane"
"#;

pub fn script(shell: &str) -> Option<&'static str> {
    match shell {
        "bash" => Some(BASH),
        "zsh" => Some(ZSH),
        "fish" => Some(FISH),
        _ => None,
    }
}
