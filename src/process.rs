    use std::time::{Duration, Instant};
    use std::process::Command;
    use std::fs::File;
    #[allow(unused_imports)]
    use yaml_rust::{Yaml,YamlLoader, YamlEmitter};
    #[allow(unused_imports)]
    use std::process::Child;

    #[derive(Debug)]
    enum Autorestart {
        TRUE,
        FALSE,
        UNEXPECTED,
    }

    #[derive(Debug)]
    pub struct Process {
        /// The Process struct represents all the informations we want
        /// to have about a single process we are supervising.
        name: String,
        command: Command,
        argv: String,
        workingdir: Option<String>,
        autostart: bool,
        env: Option<Vec<(String, String)>>,
        stdout: Option<String>,
        stderr: Option<String>,
        exitcodes: Vec<i64>,
        startretries: u64,
        umask: i64,
        autorestart: Autorestart,
        starttime: Duration,
        stopsignal: i64,
        stoptime: Duration,
        numprocs: i64,
    }


    impl Process {
        pub fn new(name: String,
                   argv: String, 
                   workingdir: Option<&str>,
                   autostart: Option<bool>,
                   env: Option<Vec<(String, String)>>,
                   stdout: Option<&str>,
                   stderr: Option<&str>,
                   exitcodes: Option<Vec<i64>>,
                   startretries: Option<i64>,
                   umask: Option<i64>,
                   autorestart: Option<&str>,
                   starttime: Option<i64>,
                   stopsignal: Option<i64>,
                   stoptime: Option<i64>,
                   numprocs: Option<i64>
        ) -> Process {
            /// Function to generate a new instance of a Process strct.
            /// Only mandatory arguments are name and command.
            /// Other arguments can be skipped by giving `None' 
            Process {
                name, 
                command: Command::new(argv.split(" ").next().unwrap()),
                argv,
                workingdir: match workingdir {
                    Some(slice) => Some(String::from(slice)),
                    None => None,
                },
                autostart: match autostart {
                    Some(value) => value,
                    None => true,
                },
                env,
                stdout: match stdout {
                    Some(slice) => Some(String::from(slice)),
                    None => None,
                },
                stderr: match stderr {
                    Some(slice) => Some(String::from(slice)),
                    None => None,
                },
                exitcodes: match exitcodes {
                    Some(v) => v,
                    None => vec![1, 2],
                },
                startretries: match startretries {
                    Some(i) => i as u64, // TODO check coherence of types i64 and u64
                    None => 3,
                },
                umask: match umask {
                    Some(i) => i,
                    None => 0700,
                },
            autorestart: match autorestart { //TODO: voir ce que c'est
                Some(slice) => if slice == "unexpected" { Autorestart::UNEXPECTED } 
                               else if slice == "true" { Autorestart::TRUE }
                               else if slice == "false"{ Autorestart::FALSE }
                               else { panic!("bad value for autorestart") }
                None => Autorestart::UNEXPECTED,
            },
                starttime:  match starttime {
                    Some(i) => Duration::from_secs(i as u64),
                    None => Duration::from_secs(1),
                },
                stopsignal: match stopsignal {
                    Some(i) => i,
                    None => 0, //TODO: mettre TERM,
                },
                stoptime:  match stoptime {
                    Some(i) => Duration::from_secs(i as u64),
                    None => Duration::from_secs(10),
                },
                numprocs:  match numprocs {
                    Some(i) => i,
                    None => 1,
                },
            }
        }
        pub fn from_yaml(name: &str, argv:&str, config: &Yaml) -> Process {
            /// Creates a Process instance from the process name and a
            /// Yaml struct representing the config options. Parses
            /// YAML into variables and calls new.

            // env is represented by a nested YAML into the current
            // config. Parsing it as a tuple of key, value.
            let env: Option<Vec<(String, String)>> = match (&config["env"]).as_hash() {
                Some(hash) => { Some(hash.iter()
                                     .map(|(name, argv)| {
                                         (String::from(name.as_str().unwrap()), 
                                          String::from(argv.as_str().unwrap()))
                                     }) //TODO: gerer les nombre
                                     .collect())
                },
                None => None,
            };

            // Exitcodes can be either one field, or many.
            let exitcodes =  match (&config["exitcodes"]).as_vec() {
                Some(v) => Some(v.iter().map(|a| {
                    a.as_i64().unwrap()})
                                .collect()),
                None => match (&config["exitcodes"]).as_i64() {
                    Some(i) => Some(vec![i]),
                    None => None,
                },
            };
            Process::new(String::from(name),
                         String::from(argv),
                         (&config["workingdir"]).as_str(),
                         (&config["autostart"]).as_bool(),
                         env,
                         (&config["stdout"]).as_str(),
                         (&config["stderr"]).as_str(),
                         exitcodes,
                         (&config["startretries"]).as_i64(),
                         (&config["umask"]).as_i64(),
                         (&config["autorestart"]).as_str(),
                         (&config["starttime"]).as_i64(),
                         (&config["stopsignal"]).as_i64(),
                         (&config["stoptime"]).as_i64(),
                         (&config["numprocs"]).as_i64(),
            )
        }

        fn add_workingdir(mut self) -> Self {
            if let Some(ref string) = self.workingdir {
                self.command.env("PWD", string);
            }
            self
        }

        fn add_args(mut self) -> Self {
            if self.argv.len() > 1 {
                let args: Vec<&str> = self.argv.split(" ").collect();
                self.command.args(&args[1..]);
            }
            self
        }

        fn add_stdout(mut self) -> Self {
            if let Some(ref string) = self.stdout {
                match File::open(string) {
                    Ok(file) => {self.command.stdout(file);},
                    Err(e) => println!("error{:?}", e),
                }
            }
            self
        }

        fn add_stderr(mut self) -> Self {
            if let Some(ref string) = self.stderr {
                match File::open(string) {
                    Ok(file) => {self.command.stderr(file);},
                    Err(e) => println!("error{:?}", e),
                }
            }
            self
        }

        fn add_env(mut self) -> Self {
            if let Some(ref vect) = self.env {
                let v: Vec<(String, String)> = vect.to_vec();
                self.command.envs(v);
            }
            self
        }

        pub fn spawn(mut self) -> Self {
            let now = Instant::now();
            let child = self.command.spawn();
            match child {
                Ok(mut child) => {
                    println!("child {} launched with pid: {:?}", self.name, child.id());
                    child.wait().unwrap();
                    let nownow = Instant::now();
                    let duree = nownow.duration_since(now);
                    println!("duree: {:?}", duree);
                    if duree < self.starttime {
                        println!("must be restart");
                    }
                }
                Err(e) => { println!("error {:?}", e);
                }
            }
            self
        }
        pub fn start(self) {
            self.add_args()
                .add_workingdir()
                .add_env()
                .add_stdout()
                .add_stderr()
                .spawn();
        }
    }
