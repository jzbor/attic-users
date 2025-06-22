use std::{collections::HashMap, fs, path::PathBuf, process::{self, Command}};

use clap::Parser;
use serde::*;


#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(rename_all = "kebab-case")]
enum CachePermissions {
    Push, Pull, Delete, Create, Configure, ConfigureCacheRetention, DestroyCache
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(rename_all = "kebab-case")]
enum CachePermissionsExtended {
    Push, Pull, Delete, Create, Configure, ConfigureCacheRetention, DestroyCache,
    All, Admin, Use
}


#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
struct CacheRule {
    pattern: String,
    permissions: Vec<CachePermissions>,
}


#[derive(clap::Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Show command but don't actually run it
    #[clap(short, long)]
    dry_run: bool,

    /// Binary to call with the corresponding args
    #[clap(short, long, default_value = "atticadm")]
    program: String,

    /// Duration of validity for the generated tokens
    #[clap(short, long, default_value = "2 years")]
    validity: String,


    /// File with user configurations in it
    #[clap(short, long, default_value = "/etc/attic-users.toml")]
    file: PathBuf,

    /// Item name
    name: String,
}


impl CachePermissions {
    fn to_atticadm_flag(&self) -> &str {
        use CachePermissions::*;
        match self {
            Push => "--push",
            Pull => "--pull",
            Delete => "--delete",
            Create => "--create-cache",
            Configure => "--configure-cache",
            ConfigureCacheRetention => "--configure-cache-retention",
            DestroyCache => "--destroy-cache",
        }
    }
}


impl From<CachePermissionsExtended>  for Vec<CachePermissions> {
    fn from(value: CachePermissionsExtended) -> Self {
        use CachePermissionsExtended::*;
        let deconstructed = match value {
            Push => vec![CachePermissions::Push],
            Pull => vec![CachePermissions::Pull],
            Delete => vec![CachePermissions::Delete],
            Create => vec![CachePermissions::Create],
            Configure => vec![CachePermissions::Configure],
            ConfigureCacheRetention => vec![CachePermissions::ConfigureCacheRetention],
            DestroyCache => vec![CachePermissions::DestroyCache],

            All => vec![
                CachePermissions::Push,
                CachePermissions::Pull,
                CachePermissions::Delete,
                CachePermissions::Create,
                CachePermissions::Configure,
                CachePermissions::ConfigureCacheRetention,
                CachePermissions::DestroyCache,
            ],
            Admin => vec![
                CachePermissions::Create,
                CachePermissions::Configure,
                CachePermissions::ConfigureCacheRetention,
                CachePermissions::DestroyCache,
            ],
            Use => vec![
                CachePermissions::Push,
                CachePermissions::Pull,
            ],
        };
        Vec::from(deconstructed)
    }
}


fn generate_command(config: &HashMap<String, Vec<CacheRule>>, args: &Args) -> (String, Command) {
    let mut cmd = Command::new(&args.program);
    let user = &args.name;
    if let Some(rules) = config.get(user) {
        cmd.args(["make-token", "--sub", &user, "--validity", &args.validity]);
        for rule in rules {
            for perm in &rule.permissions {
                cmd.args([perm.to_atticadm_flag(), &rule.pattern]);
            }
        }
        (user.to_owned(), cmd)
    } else {
        die("Could not find rules", &format!("No such user '{}' in files", user));
    }
}

fn die(err: &str, reason: &str) -> !{
    eprintln!("{}: {}", err, reason);
    process::exit(1)
}

fn main() {
    let args = Args::parse();

    let contents = match fs::read_to_string(&args.file) {
        Ok(contents) => contents,
        Err(e) => die("Unable to read config file", &e.to_string()),
    };
    let config: HashMap<String, HashMap<String, Vec<CachePermissionsExtended>>> = match toml::from_str(&contents) {
        Ok(config) => config,
        Err(e) => die("Unable to parse config", &e.to_string()),
    };

    let config: HashMap<String, Vec<CacheRule>> = config.into_iter()
        .map(|(name, v)| {
            let rules = v.into_iter()
                .map(|(pattern, extended)| {
                    let permissions = extended.into_iter().flat_map(|x| Vec::from(x)).collect();
                    CacheRule { pattern, permissions }
                })
                .collect();
            (name, rules)
        }).collect();

    let (user, mut cmd) = generate_command(&config, &args);

    println!("\n=> Fetching token for '{}'", user);
    if args.dry_run {
        println!("{:?}", cmd);
    } else {
        match cmd.status() {
            Ok(_) => (),
            Err(e) => die("Unable execute command", &e.to_string()),
        };
    }
}
