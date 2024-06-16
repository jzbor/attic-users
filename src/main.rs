use std::{fs, path::PathBuf, process::{self, Command}};

use clap::Parser;
use serde::*;

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(rename_all = "lowercase")]
enum CachePermissions {
    Push, Pull, Delete, Create, Configure
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
struct UserConfig {
    name: String,
    rules: Vec<CacheRule>,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
struct CacheRule {
    pattern: String,
    permissions: Vec<CachePermissions>,
}


#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
struct ModeGroup {
    /// Print example to stdout
    #[clap(short, long)]
    example: bool,

    file: Option<PathBuf>,
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    dry_run: bool,

    #[clap(short, long, default_value = "atticadm")]
    program: String,

    #[clap(short, long, default_value = "3 years")]
    validity: String,

    #[clap(flatten)]
    mode: ModeGroup,
}


impl UserConfig {
    fn example() -> Vec<Self> {
        use CachePermissions::*;
        vec![
            UserConfig {
                name: String::from("alice"),
                rules: vec![
                    CacheRule {
                        pattern: String::from("alice-*"),
                        permissions: vec![Push, Pull, Create, Delete],
                    },
                ],
            },
            UserConfig {
                name: String::from("bob"),
                rules: vec![
                    CacheRule {
                        pattern: String::from("bob-*"),
                        permissions: vec![Push, Pull, Create, Delete],
                    },
                ],
            },
        ]
    }
}

impl CachePermissions {
    fn to_atticadm_flag(&self) -> &str {
        use CachePermissions::*;
        match self {
            Push => "--push",
            Pull => "--pull",
            Delete => "--delete",
            Create => "--create-cache",
            Configure => "--configure",
        }
    }
}

fn generate_commands(config: &Vec<UserConfig>, args: &Args) -> Vec<(String, Command)> {
    let mut commands = vec![];
    for user in config {
        let mut cmd = Command::new(&args.program);
        cmd.args(["make-token", "--sub", &user.name, "--validity", &args.validity]);
        for rule in &user.rules {
            for perm in &rule.permissions {
                cmd.args([perm.to_atticadm_flag(), &rule.pattern]);
            }
        }
        commands.push((user.name.clone(), cmd));
    }
    commands
}

fn die(err: &str, reason: &str) -> !{
    eprintln!("{}: {}", err, reason);
    process::exit(1)
}

fn main() {
    let args = Args::parse();

    if args.mode.example {
        println!("{}", serde_yaml::to_string(&UserConfig::example()).unwrap());
    } else if let Some(file) = &args.mode.file {
        let contents = match fs::read_to_string(file) {
            Ok(contents) => contents,
            Err(e) => die("Unable to read file", &e.to_string()),
        };
        let config: Vec<UserConfig> = match serde_yaml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => die("Unable to parse config", &e.to_string()),
        };
        let mut commands = generate_commands(&config, &args);

        for (user, cmd) in &mut commands {
            println!("\n=> Registering '{}'", user);
            if args.dry_run {
                println!("{:?}", cmd);
            } else {
                match cmd.status() {
                    Ok(_) => (),
                    Err(e) => die("Unable execute command", &e.to_string()),
                };
            }
        }
    }
}
