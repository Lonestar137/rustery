use clap::Parser;
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::vec;

/*
    TODO:
    Add lua config file support(wezterm).
*/

trait Builder {
    fn new(config: &CommandlineArgs) -> Self;
    fn glob(&mut self, basepath: PathBuf, file_extension: &str);
    fn read_edges(&mut self);
    fn make_dependency_tree(&mut self);
    fn make_build_queue(&mut self);
    fn build_local_image(&self, image: &String);
    fn pull_image(&self, image: &String);
    fn build(&self);
}

struct RegistryBuilder {
    config: CommandlineArgs,
    files: Vec<PathBuf>,
    edges: Vec<(String, String)>,
    dep_tree: HashMap<String, Vec<String>>,
    build_queue: VecDeque<String>,
}

impl Builder for RegistryBuilder {
    fn new(config: &CommandlineArgs) -> Self {
        let mut rb = Self {
            config: config.to_owned(),
            files: vec![],
            edges: vec![],
            dep_tree: HashMap::new(),
            build_queue: VecDeque::new(),
        };
        rb.glob(config.basepath.to_owned(), &config.extension);
        rb.read_edges();
        rb.make_dependency_tree();
        rb.make_build_queue();
        return rb;
    }

    fn glob(&mut self, basepath: PathBuf, file_extension: &str) {
        let path_iter = fs::read_dir(basepath);
        match path_iter {
            Ok(dir_iter) => {
                for entry_result in dir_iter {
                    let msg = format!("Unable to process file. {:?}", entry_result);
                    let entry = entry_result.expect(&msg);
                    let is_dir = entry
                        .file_type()
                        .expect("Unable to determine directory status.")
                        .is_dir();

                    if is_dir {
                        self.glob(entry.path(), file_extension)
                    } else {
                        let path = entry.path();
                        let has_extension = entry
                            .file_name()
                            .into_string()
                            .expect("Failed to check file extension.")
                            .ends_with(file_extension);
                        if has_extension {
                            self.files.push(path);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Unable to read filepath. {:?}", e);
            }
        }
    }

    fn read_edges(&mut self) {
        let mut edges: Vec<(String, String)> = vec![];
        for file in &self.files {
            let error_msg = format!("Failed to read file: {:?}", file);
            let file_contents = fs::read_to_string(file).expect(&error_msg);
            let from_files = file_contents
                .split("\n")
                .filter(|x| x.starts_with("FROM"))
                .collect::<Vec<&str>>();

            let file_string = file.to_str().expect("Failed to convert PathBuf to string.");
            for ele in from_files {
                // let image = ele.split(" ").unwrap().1.to_string();
                let image = ele.split_whitespace().collect::<Vec<&str>>()[1].to_string();
                let file_image = "localhost".to_string()
                    + "/"
                    + &&file_string
                        .replacen("./", "", 1)
                        .replace("__", ":")
                        .replace(".", "")
                        .rsplit_once(&self.config.extension)
                        .unwrap()
                        .0;
                edges.push((file_image, image));
            }
        }
        self.edges = edges;
    }

    fn make_dependency_tree(&mut self) {
        let mut map = HashMap::new();
        for (image, _) in &self.edges {
            let mut deps = vec![];
            for (i, d) in &self.edges {
                if image == i {
                    deps.push(d.to_owned());
                }
            }
            map.insert(image.to_owned(), deps);
        }
        self.dep_tree = map;
    }

    fn make_build_queue(&mut self) {
        // Add base cases
        let mut unique_values: VecDeque<String> = self
            .edges
            .iter()
            .map(|(_, v)| v)
            .filter(|v| self.edges.iter().all(|(x, _)| &x != v))
            .map(|v| v.to_string())
            .collect();

        self.build_queue.append(&mut unique_values);

        // Add everything else to the build queue.
        fn add_nodes(
            dep_tree: HashMap<String, Vec<String>>,
            build_queue: &mut VecDeque<String>,
            limit: u8,
        ) {
            let mut not_added = HashMap::new();
            for (image, deps) in &dep_tree {
                let contains_deps = deps.iter().all(|i| build_queue.contains(i));
                if contains_deps {
                    build_queue.push_back(image.to_owned());
                } else {
                    not_added.insert(image.to_owned(), deps.to_owned());
                }
            }
            if limit < 1 {
                println!("Failed to find dependencies for: {:?}", not_added);
                return;
            }
            if !not_added.is_empty() {
                add_nodes(not_added, build_queue, limit - 1);
            }
        }

        add_nodes(self.dep_tree.to_owned(), &mut self.build_queue, 20);
    }

    fn build_local_image(&self, image: &String) {
        let local_filepath = image.replacen(":", "__", 1).replacen("localhost", ".", 1)
            + "."
            + &self.config.extension;
        let mut build_args = vec!["build", "--file", &local_filepath, "--tag", &image];
        let mut tag_args = vec!["tag", image];
        let remote_image = match &self.config.registry {
            Some(registry) => image.replacen("localhost", registry.as_str(), 1),
            None => image.replacen("localhost", "localhost", 1),
        };

        if &self.config.client == "podman" {
            build_args.extend(["--format", "docker"]);
        }

        // if &self.config.registry != "localhost" {
        if self.config.registry.is_some() {
            tag_args.extend(vec![remote_image.as_str()]);
        }

        // Add build context
        let build_context = ".";
        build_args.extend(vec![build_context]);

        if self.config.dryrun {
            println!("{} {}", &self.config.client, build_args.join(" "));
            if self.config.registry.is_some() {
                println!("{} {}", &self.config.client, tag_args.join(" "));
            }
        } else {
            println!("Building: {}", image);
            // TODO: test this part out.
            let build_cmd = Command::new(&self.config.client)
                .args(build_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to run build command.");
            show_process_io(build_cmd);
            if self.config.registry.is_some() {
                let tag_cmd = Command::new(&self.config.client)
                    .args(tag_args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("Failed to run tag command.");

                show_process_io(tag_cmd);
            }
        }
    }

    fn pull_image(&self, image: &String) {
        let pull_args = vec!["pull", image];

        if self.config.dryrun {
            println!("{} {}", &self.config.client, pull_args.join(" "));
        } else {
            let pull_command = Command::new(&self.config.client)
                .args(pull_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to run pull command.");

            show_process_io(pull_command);
        }
    }

    fn build(&self) {
        for image in &self.build_queue {
            if image.starts_with("localhost") {
                self.build_local_image(image);
            } else {
                self.pull_image(image);
            }
        }
    }
}

fn show_process_io(mut process: Child) {
    // Capture the stdout and stderr streams
    let mut stdout = process.stdout.take().unwrap();
    let mut stderr = process.stderr.take().unwrap();

    // Spawn threads to read from the stdout and stderr streams and print the output
    let stdout_thread = std::thread::spawn(move || {
        std::io::copy(&mut stdout, &mut std::io::stdout()).expect("Failed to write to stdout");
    });

    let stderr_thread = std::thread::spawn(move || {
        std::io::copy(&mut stderr, &mut std::io::stderr()).expect("Failed to write to stderr");
    });

    // Wait for the command to finish and collect the exit status
    let exit_status = process
        .wait()
        .expect("Failed to wait for command to finish");

    // Wait for the stdout and stderr threads to finish
    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();

    println!("Command exited with status: {}", exit_status);
}

/// Automatically orchestrates container builds.
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct CommandlineArgs {
    /// CLI container client to use.
    #[arg(short, long, default_value = "podman")]
    client: String,

    /// Directory to scan for containerfiles in.
    #[arg(short, long, default_value = ".")]
    basepath: PathBuf,

    /// File extension of containerfiles.
    #[arg(short, long, default_value = "docker")]
    extension: String,

    /// Remote registry to push built images to.
    #[arg(short, long, default_value = None)]
    registry: Option<String>,

    /// Dryrun
    #[arg(short, long, default_value_t = false)]
    dryrun: bool,
}

// TODO: add support for a .registry file.
/*
Features:
    - By default, rustery uses the .registry file as the context path starting point.
    - Can specify various different options in the config, CLI always overwrites though.
*/
fn main() {
    let stdin_args: Vec<String> = env::args().collect();
    let config_file = Path::new("./.rustery");
    if stdin_args.len() == 1 && config_file.exists() {
        println!("CONFIG FILE");
    } else {
        let args = CommandlineArgs::parse();
        let builder = RegistryBuilder::new(&args);
        builder.build();
    }
}
