//! Shared Bash-command lexer for file-op intent detection.
//!
//! Intentionally narrow: we accept false negatives, never false positives.
//! Recognises `mv`, `cp`, `rm`, `git mv`, shell redirects, and `sed -i`.
//!
//! Any shell path too exotic for this lexer is caught downstream by the
//! SHA ladder (PROPOSAL §D.3), which attributes the change to `human`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellIntent {
    Move { from: String, to: String },
    Remove { path: String },
    Copy { from: String, to: String },
    RedirectWrite { path: String, append: bool },
    SedInPlace { path: String },
    PythonWriteBestEffort { path: String },
}

pub fn detect_intents(cmd: &str) -> Vec<ShellIntent> {
    let mut out = Vec::new();
    // Split on `&&`, `||`, `;` to handle compound commands.
    for piece in split_top_level(cmd) {
        out.extend(detect_one(piece.trim()));
    }
    out
}

fn split_top_level(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Avoid splitting inside single-quoted strings.
        if bytes[i] == b'\'' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'\'' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'&' && i + 1 < bytes.len() && bytes[i + 1] == b'&' {
            out.push(&s[start..i]);
            i += 2;
            start = i;
            continue;
        }
        if bytes[i] == b'|' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            out.push(&s[start..i]);
            i += 2;
            start = i;
            continue;
        }
        if bytes[i] == b';' {
            out.push(&s[start..i]);
            i += 1;
            start = i;
            continue;
        }
        i += 1;
    }
    out.push(&s[start..]);
    out
}

fn detect_one(s: &str) -> Vec<ShellIntent> {
    let toks = tokenize(s);
    if toks.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();

    // mv / git mv
    let is_mv = toks[0] == "mv" || (toks[0] == "git" && toks.get(1).map(|s| s.as_str()) == Some("mv"));
    if is_mv {
        let args: Vec<&str> = toks
            .iter()
            .skip(if toks[0] == "git" { 2 } else { 1 })
            .filter(|a| !a.starts_with('-'))
            .map(String::as_str)
            .collect();
        if args.len() == 2 {
            out.push(ShellIntent::Move {
                from: args[0].into(),
                to: args[1].into(),
            });
        }
    }

    if toks[0] == "cp" {
        let args: Vec<&str> = toks
            .iter()
            .skip(1)
            .filter(|a| !a.starts_with('-'))
            .map(String::as_str)
            .collect();
        if args.len() == 2 {
            out.push(ShellIntent::Copy {
                from: args[0].into(),
                to: args[1].into(),
            });
        }
    }

    if toks[0] == "rm" {
        for a in toks.iter().skip(1).filter(|a| !a.starts_with('-')) {
            out.push(ShellIntent::Remove { path: a.clone() });
        }
    }

    // sed -i <expr> <file>
    if toks[0] == "sed" {
        let has_inplace = toks.iter().any(|t| t.starts_with("-i"));
        if has_inplace {
            if let Some(last) = toks.last() {
                if !last.starts_with('-') {
                    out.push(ShellIntent::SedInPlace { path: last.clone() });
                }
            }
        }
    }

    // shell redirect: "foo > path" or "tee path" or "foo >> path"
    if let Some(idx) = toks.iter().position(|t| t == ">" || t == ">>") {
        if let Some(p) = toks.get(idx + 1) {
            let append = toks[idx] == ">>";
            out.push(ShellIntent::RedirectWrite {
                path: p.clone(),
                append,
            });
        }
    }
    if toks[0] == "tee" {
        for a in toks.iter().skip(1).filter(|a| !a.starts_with('-')) {
            out.push(ShellIntent::RedirectWrite {
                path: a.clone(),
                append: false,
            });
        }
    }

    // Best-effort: python -c "...open('x','w').write(...)..."
    if (toks[0] == "python" || toks[0] == "python3") && toks.iter().any(|t| t == "-c") {
        if let Some(c_idx) = toks.iter().position(|t| t == "-c") {
            if let Some(script) = toks.get(c_idx + 1) {
                if let Some(path) = extract_python_open_write(script) {
                    out.push(ShellIntent::PythonWriteBestEffort { path });
                }
            }
        }
    }

    out
}

fn extract_python_open_write(script: &str) -> Option<String> {
    // Very tight: match open("path", "w"...) or open('path', 'w'...).
    // False negatives are OK (SHA ladder catches it); false positives are not.
    let patterns = [
        ("open(\"", '"'),
        ("open('", '\''),
    ];
    for (prefix, end) in patterns {
        if let Some(i) = script.find(prefix) {
            let tail = &script[i + prefix.len()..];
            if let Some(j) = tail.find(end) {
                let path = &tail[..j];
                let rest = &tail[j..];
                // Require the second arg to be "w"/"a".
                if rest.contains("\"w\"")
                    || rest.contains("'w'")
                    || rest.contains("\"a\"")
                    || rest.contains("'a'")
                {
                    return Some(path.to_string());
                }
            }
        }
    }
    None
}

fn tokenize(s: &str) -> Vec<String> {
    // Minimal shell tokenizer: preserves `"..."` and `'...'` strings as one
    // token with quotes stripped. Not a full parser.
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {
                if !cur.is_empty() {
                    toks.push(std::mem::take(&mut cur));
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '"' {
                        break;
                    }
                    if next == '\\' {
                        if let Some(&esc) = chars.peek() {
                            chars.next();
                            s.push(esc);
                        }
                        continue;
                    }
                    s.push(next);
                }
                cur.push_str(&s);
            }
            '\'' => {
                let mut s = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\'' {
                        break;
                    }
                    s.push(next);
                }
                cur.push_str(&s);
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mv() {
        let ints = detect_intents("mv a.txt b.txt");
        assert_eq!(
            ints,
            vec![ShellIntent::Move {
                from: "a.txt".into(),
                to: "b.txt".into()
            }]
        );
    }

    #[test]
    fn detects_git_mv() {
        let ints = detect_intents("git mv old.rs new.rs");
        assert_eq!(
            ints,
            vec![ShellIntent::Move {
                from: "old.rs".into(),
                to: "new.rs".into()
            }]
        );
    }

    #[test]
    fn detects_rm() {
        let ints = detect_intents("rm -rf dist");
        assert_eq!(ints, vec![ShellIntent::Remove { path: "dist".into() }]);
    }

    #[test]
    fn detects_redirect() {
        let ints = detect_intents("echo hi > hi.txt");
        assert_eq!(
            ints,
            vec![ShellIntent::RedirectWrite {
                path: "hi.txt".into(),
                append: false
            }]
        );
    }

    #[test]
    fn detects_sed_inplace() {
        let ints = detect_intents("sed -i s/a/b/ file.txt");
        assert_eq!(
            ints,
            vec![ShellIntent::SedInPlace {
                path: "file.txt".into()
            }]
        );
    }

    #[test]
    fn detects_python_open_write() {
        let ints = detect_intents(r#"python -c "open('x.txt','w').write('hi')""#);
        assert_eq!(
            ints,
            vec![ShellIntent::PythonWriteBestEffort {
                path: "x.txt".into()
            }]
        );
    }

    #[test]
    fn compound_commands() {
        let ints = detect_intents("mv a b && rm c");
        assert!(ints.contains(&ShellIntent::Move {
            from: "a".into(),
            to: "b".into()
        }));
        assert!(ints.contains(&ShellIntent::Remove { path: "c".into() }));
    }
}
