use freedesktop_entry_parser::{errors::ParseError, parse_entry, Entry};
use std::collections::HashMap;
use std::fs::{read_dir, DirEntry};
use std::path::PathBuf;

fn main() {
    run()
}
fn run() {
    let term_prefix = "alacrity -e ";
    let data = desktop_entries()
        .filter_map(|r| match r {
            Ok(a) => Some(a),
            Err(er) => {
                eprintln!("Error: {}", er);
                None
            }
        })
        .map(|e| parse_ent(&e));

    let (root_mode, submodes) = data.map(|ent| map_ent(ent, term_prefix)).fold(
        (Vec::new(), HashMap::new()),
        |(mut root_mode, mut submodes), (root_ent, mode_data)| {
            root_mode.push(root_ent);
            if let Some((mode_name, mode_ents)) = mode_data {
                assert!(submodes.insert(mode_name, mode_ents).is_none());
            }
            (root_mode, submodes)
        },
    );
    println!("[[command]]");
    println!("message = \"open\"");
    println!("mode = \"open app\"");
    println!("");
    println!("[[mode]]");
    println!("name = \"open app\"");
    println!("");
    for cmd in root_mode {
        println!("[[mode.command]]");
        println!("message = \"{}\"", cmd.message);
        if let Some(exec) = cmd.exec {
            println!("command = \"{}\"", exec);
        }
        if let Some(md) = cmd.mode {
            println!("mode = \"{}\"", md);
        }
        println!("");
    }
    println!("");
    let mut submodes: Vec<_> = submodes.into_iter().collect();
    submodes.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (mode_name, mode_ents) in submodes {
        println!("[[mode]]");
        println!("name = \"{}\"", mode_name);
        println!("");
        for cmd in mode_ents {
            println!("[[mode.command]]");
            println!("message = \"{}\"", cmd.message);
            if let Some(exec) = cmd.exec {
                println!("command = \"{}\"", exec);
            }
            if let Some(md) = cmd.mode {
                println!("mode = \"{}\"", md);
            }
            println!("");
        }
    }
}

fn map_ent(
    ent: MyWrapper,
    term_prefix: &str,
) -> (CommandEntry, Option<(String, Vec<CommandEntry>)>) {
    if ent.actions.is_empty() {
        let exec = make_exec_str(ent.exec, ent.term, term_prefix).into();
        let mode = None;
        let cmd_ent = CommandEntry {
            message: ent.name,
            exec,
            mode,
        };
        return (cmd_ent, None);
    }
    let modename = format!("{} actions", ent.name);
    let mode = Some(modename.clone());
    let cmd_ent = CommandEntry {
        message: ent.name,
        exec: None,
        mode,
    };
    let exec = make_exec_str(ent.exec, ent.term, term_prefix).into();
    let default_action = CommandEntry {
        message: "".to_owned(),
        exec,
        mode: None,
    };
    let actions = ent.actions.into_iter().map(|act| {
        let message = act.name;
        let exec = make_exec_str(act.exec, act.term, term_prefix).into();
        CommandEntry {
            message,
            exec,
            mode: None,
        }
    });
    let submode_ents: Vec<_> = std::iter::once(default_action).chain(actions).collect();
    (cmd_ent, Some((modename, submode_ents)))
}

struct CommandEntry {
    message: String,
    exec: Option<String>,
    mode: Option<String>,
}

fn parse_ent(ent: &Entry) -> MyWrapper {
    let mut name = None;
    let mut exec = None;
    let mut try_exec = None;
    let mut term = None;
    let mut actions = Vec::new();

    for sec in ent.sections() {
        if sec.name() == "Desktop Entry" {
            assert!(name.is_none());
            assert!(exec.is_none());
            assert!(term.is_none());
            assert!(try_exec.is_none());
            name = sec
                .attr_with_param("Name", "en")
                .or_else(|| sec.attr("Name"))
                .map(|s| s.to_owned());
            term = sec.attr("Terminal").map(|s| match s {
                "true" => true,
                "false" => false,
                other => panic!("Invalid terminal value: {}", other),
            });
            exec = sec.attr("Exec").map(|s| s.to_owned());
            try_exec = sec.attr("TryExec").map(|s| s.to_owned());
        } else if sec.name().starts_with("Desktop Action") {
            let atitle = sec.name().trim_start_matches("Desktop Action ").to_owned();
            let aname = sec
                .attr_with_param("Name", "en")
                .or_else(|| sec.attr("Name"))
                .map(|s| s.to_owned())
                .unwrap();
            let aexec = sec.attr("Exec").map(|s| s.to_owned()).unwrap();
            let aterm = sec.attr("Terminal").map(|s| match s {
                "true" => true,
                "false" => false,
                other => panic!("Invalid terminal value: {}", other),
            });
            let ndisp = sec.attr("NoDisplay").map(|s| match s {
                "true" => true,
                "false" => false,
                other => panic!("Invalid terminal value: {}", other),
            });
            let aterm = aterm.or(ndisp);
            let atexec = sec.attr("TryExec").map(|s| s.to_owned());
            let action = MyActionWrapper {
                title: atitle,
                name: aname,
                exec: aexec,
                try_exec: atexec,
                term: aterm,
            };
            actions.push(action);
        }
    }
    MyWrapper {
        name: name.unwrap(),
        exec: exec.unwrap(),
        term,
        actions,
        try_exec,
    }
}

fn make_exec_str(raw: String, term: Option<bool>, term_prefix: &str) -> String {
    let term = term.unwrap_or(false);
    let raw = raw.trim().trim_matches('"').trim();
    if term {
        format!("{} {}", term_prefix, raw)
    } else {
        raw.to_owned()
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct MyWrapper {
    name: String,
    exec: String,
    term: Option<bool>,
    try_exec: Option<String>,
    actions: Vec<MyActionWrapper>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct MyActionWrapper {
    title: String,
    name: String,
    exec: String,
    try_exec: Option<String>,
    term: Option<bool>,
}

#[derive(Debug)]
enum MyError {
    Io(std::io::Error),
    ParseError(ParseError),
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyError::Io(e) => e.fmt(f),
            MyError::ParseError(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for MyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MyError::Io(e) => e.source(),
            MyError::ParseError(e) => e.source(),
        }
    }
}

fn desktop_entries() -> impl Iterator<Item = Result<Entry, MyError>> {
    desktop_entry_paths().map(|r| match r {
        Ok(p) => parse_entry(p.path()).map_err(MyError::ParseError),
        Err(e) => Err(MyError::Io(e)),
    })
}

fn desktop_entry_paths() -> impl Iterator<Item = Result<DirEntry, std::io::Error>> {
    desktop_entry_dirs()
        .flat_map(|dir| match read_dir(dir) {
            Ok(r) => EitherIter::Left(r),
            Err(e) => EitherIter::Right(std::iter::once(Err(e))),
        })
        .filter(|res| match res {
            Ok(f) => f.path().extension() == Some("desktop".as_ref()),
            Err(_) => true,
        })
}

fn desktop_entry_dirs() -> impl Iterator<Item = PathBuf> {
    xdg_data_home()
        .into_iter()
        .chain(xdg_data_sys())
        .map(|mut p| {
            p.push("applications");
            p
        })
}

fn xdg_data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(|s| PathBuf::from(s))
        .or_else(|| {
            let raw_home = std::env::var_os("HOME")?;
            let mut res = PathBuf::from(raw_home);
            res.push(".local/share");
            Some(res)
        })
}

fn xdg_data_sys() -> impl Iterator<Item = PathBuf> {
    std::env::var_os("XDG_DATA_DIRS")
        .map(|s| std::env::split_paths(&s).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["/usr/local/share".into(), "/usr/share".into()])
        .into_iter()
}

enum EitherIter<Itm, Lft: Iterator<Item = Itm>, Rgt: Iterator<Item = Itm>> {
    Left(Lft),
    Right(Rgt),
}

impl<Itm, Lft: Iterator<Item = Itm>, Rgt: Iterator<Item = Itm>> Iterator
    for EitherIter<Itm, Lft, Rgt>
{
    type Item = Itm;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EitherIter::Left(inner) => inner.next(),
            EitherIter::Right(inner) => inner.next(),
        }
    }
}
