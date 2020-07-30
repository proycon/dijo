use std::fmt;

use cursive::event::{Event, EventResult, Key};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::view::Resizable;
use cursive::views::{EditView, LinearLayout, OnEventView, TextView};
use cursive::Cursive;

use crate::{app::App, CONFIGURATION};

static COMMANDS: &'static [&'static str] = &[
    "add",
    "add-auto",
    "delete",
    "track-up",
    "track-down",
    "month-prev",
    "month-next",
    "quit",
    "write",
    "help",
];

fn get_command_completion(prefix: &str) -> Option<String> {
    let first_match = COMMANDS.iter().filter(|&x| x.starts_with(prefix)).next();
    return first_match.map(|&x| x.into());
}

fn get_habit_completion(prefix: &str, habit_names: &[String]) -> Option<String> {
    let first_match = habit_names.iter().filter(|&x| x.starts_with(prefix)).next();
    eprintln!("{:?}| {:?}", prefix, first_match);
    return first_match.map(|x| x.into());
}

pub fn open_command_window(s: &mut Cursive) {
    let habit_list: Vec<String> = s
        .call_on_name("Main", |view: &mut App| {
            return view.list_habits();
        })
        .unwrap();
    let style = ColorStyle::new(Color::Dark(BaseColor::Black), Color::Dark(BaseColor::White));
    let command_window = OnEventView::new(
        EditView::new()
            .filler(" ")
            .on_submit(call_on_app)
            .style(style),
    )
    .on_event_inner(
        Event::Key(Key::Tab),
        move |view: &mut EditView, _: &Event| {
            let contents = view.get_content();
            if !contents.contains(" ") {
                let completion = get_command_completion(&*contents);
                if let Some(c) = completion {
                    let cb = view.set_content(c);
                    return Some(EventResult::Consumed(Some(cb)));
                };
                return None;
            } else {
                let word = contents.split(' ').last().unwrap();
                let completion = get_habit_completion(word, &habit_list);
                eprintln!("{:?} | {:?}", completion, contents);
                if let Some(c) = completion {
                    let cb = view.set_content(format!("{}", contents) + &c[word.len()..]);
                    return Some(EventResult::Consumed(Some(cb)));
                };
                return None;
            }
        },
    )
    .fixed_width(CONFIGURATION.view_width * CONFIGURATION.grid_width);
    s.call_on_name("Frame", |view: &mut LinearLayout| {
        let mut commandline = LinearLayout::horizontal()
            .child(TextView::new(":"))
            .child(command_window);
        commandline.set_focus_index(1);
        view.add_child(commandline);
        view.set_focus_index(1);
    });
}

fn call_on_app(s: &mut Cursive, input: &str) {
    // things to do after recieving the command
    // 1. parse the command
    // 2. clean existing command messages
    // 3. remove the command window
    // 4. handle quit command
    s.call_on_name("Main", |view: &mut App| {
        let cmd = Command::from_string(input);
        view.clear_message();
        view.parse_command(cmd);
    });
    s.call_on_name("Frame", |view: &mut LinearLayout| {
        view.set_focus_index(0);
        view.remove_child(view.get_focus_index());
    });

    // special command that requires access to
    // our main cursive object, has to be parsed again
    // here
    // TODO: fix this somehow
    if let Ok(Command::Quit) = Command::from_string(input) {
        s.quit();
    }
}

#[derive(PartialEq, Debug)]
pub enum Command {
    Add(String, Option<u32>, bool),
    MonthPrev,
    MonthNext,
    Delete(String),
    TrackUp(String),
    TrackDown(String),
    Help(Option<String>),
    Write,
    Quit,
    Blank,
}

#[derive(Debug)]
pub enum CommandLineError {
    InvalidCommand(String),     // command name
    NotEnoughArgs(String, u32), // command name, required no. of args
}

impl std::error::Error for CommandLineError {}

impl fmt::Display for CommandLineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandLineError::InvalidCommand(s) => write!(f, "Invalid command: `{}`", s),
            CommandLineError::NotEnoughArgs(s, n) => {
                write!(f, "Command `{}` requires atleast {} argument(s)!", s, n)
            }
        }
    }
}

type Result<T> = std::result::Result<T, CommandLineError>;

impl Command {
    pub fn from_string<P: AsRef<str>>(input: P) -> Result<Command> {
        let mut strings: Vec<&str> = input.as_ref().trim().split(' ').collect();
        if strings.is_empty() {
            return Ok(Command::Blank);
        }

        let first = strings.first().unwrap().to_string();
        let args: Vec<String> = strings.iter_mut().skip(1).map(|s| s.to_string()).collect();
        let mut _add = |auto: bool, first: String| {
            return parse_add(first, args.clone(), auto);
        };

        match first.as_ref() {
            "add" | "a" => _add(false, first),
            "add-auto" | "aa" => _add(true, first),
            "delete" | "d" => {
                if args.is_empty() {
                    return Err(CommandLineError::NotEnoughArgs(first, 1));
                }
                return Ok(Command::Delete(args[0].to_string()));
            }
            "track-up" | "tup" => {
                if args.is_empty() {
                    return Err(CommandLineError::NotEnoughArgs(first, 1));
                }
                return Ok(Command::TrackUp(args[0].to_string()));
            }
            "track-down" | "tdown" => {
                if args.is_empty() {
                    return Err(CommandLineError::NotEnoughArgs(first, 1));
                }
                return Ok(Command::TrackDown(args[0].to_string()));
            }
            "h" | "?" | "help" => {
                if args.is_empty() {
                    return Ok(Command::Help(None));
                }
                return Ok(Command::Help(Some(args[0].to_string())));
            }
            "mprev" | "month-prev" => return Ok(Command::MonthPrev),
            "mnext" | "month-next" => return Ok(Command::MonthNext),
            "q" | "quit" => return Ok(Command::Quit),
            "w" | "write" => return Ok(Command::Write),
            "" => return Ok(Command::Blank),
            s => return Err(CommandLineError::InvalidCommand(s.into())),
        }
    }
}

fn parse_add(verb: String, args: Vec<String>, auto: bool) -> Result<Command> {
    if args.is_empty() {
        return Err(CommandLineError::NotEnoughArgs(verb, 1));
    }

    let mut pos = 1;
    let mut acc = "".to_owned();
    let mut new_goal: Option<u32> = None;
    for s1 in args {
        if pos == 1 {
            acc.push_str(&s1);
        } else {
            if let Ok(n) = s1.parse::<u32>() {
                new_goal = Some(n);
            } else {
                acc.push(' ');
                acc.push_str(&s1);
            }
        }
        pos = pos + 1;
    }

    return Ok(Command::Add(acc, new_goal, auto));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_add_command() {
        let result = Command::from_string("add eat 2");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Add(name, goal, auto) => {
                assert_eq!(name, "eat");
                assert_eq!(goal.unwrap(), 2);
                assert_eq!(auto, false);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_add_command_without_goal() {
        let result = Command::from_string("add eat");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Add(name, goal, auto) => {
                assert_eq!(name, "eat");
                assert!(goal.is_none());
                assert_eq!(auto, false);
            }
            _ => panic!(),
        }
    }

    // #[test]
    fn parse_add_command_with_long_name() {
        let result = Command::from_string("add \"eat healthy\" 5");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Add(name, goal, auto) => {
                assert_eq!(name, "eat healthy");
                assert_eq!(goal.unwrap(), 5);
                assert_eq!(auto, false);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_add_auto_command() {
        let result = Command::from_string("add-auto eat 2");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Add(name, goal, auto) => {
                assert_eq!(name, "eat");
                assert_eq!(goal.unwrap(), 2);
                assert_eq!(auto, true);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_delete_command() {
        let result = Command::from_string("delete eat");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Delete(name) => {
                assert_eq!(name, "eat");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_track_up_command() {
        let result = Command::from_string("track-up eat");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::TrackUp(name) => {
                assert_eq!(name, "eat");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_track_down_command() {
        let result = Command::from_string("track-down eat");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::TrackDown(name) => {
                assert_eq!(name, "eat");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_help_command() {
        let result = Command::from_string("help add");

        assert!(result.is_ok());
        match result.unwrap() {
            Command::Help(name) => {
                assert_eq!(name.unwrap(), "add");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn parse_month_prev_command() {
        let result = Command::from_string("mprev");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::MonthPrev);
    }

    #[test]
    fn parse_month_next_command() {
        let result = Command::from_string("mnext");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::MonthNext);
    }

    #[test]
    fn parse_quit_command() {
        let result = Command::from_string("q");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Quit);
    }

    #[test]
    fn parse_write_command() {
        let result = Command::from_string("w");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Write);
    }

    #[test]
    fn parse_no_command() {
        let result = Command::from_string("");

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Blank);
    }
}
