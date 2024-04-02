use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::vec;

/*
    TODO:
    Add lua config file support(wezterm).
*/

trait Builder {
    fn new(
        registry: &str,
        client: &str,
        dryrun: bool,
        basepath: PathBuf,
        file_extension: &str,
    ) -> Self;
    fn glob(&mut self, basepath: PathBuf, file_extension: &str);
    fn read_edges(&mut self);
    fn make_dependency_tree(&mut self);
    fn make_build_queue(&mut self);
    fn build(&self);
}

struct RegistryBuilder {
    file_extension: String,
    registry: String,
    client: String,
    dryrun: bool,
    files: Vec<PathBuf>,
    edges: Vec<(String, String)>,
    dep_tree: HashMap<String, Vec<String>>,
    build_queue: VecDeque<String>,
}

impl Builder for RegistryBuilder {
    fn new(
        registry: &str,
        client: &str,
        dryrun: bool,
        basepath: PathBuf,
        file_extension: &str,
    ) -> Self {
        let mut rb = Self {
            file_extension: file_extension.to_string(),
            registry: registry.to_string(),
            client: client.to_string(),
            dryrun,
            files: vec![],
            edges: vec![],
            dep_tree: HashMap::new(),
            build_queue: VecDeque::new(),
        };
        rb.glob(basepath, file_extension);
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
                    + &&file_string
                        .replacen("./", "/", 1)
                        .replace("__", ":")
                        .replace(".", "")
                        .rsplit_once(&self.file_extension)
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

    fn build(&self) {
        for image in &self.build_queue {
            let mut build_args = vec!["build", "--file", image];
            let mut tag_args = vec!["tag", image];
            let remote_image = image.replacen("localhost", &self.registry, 1);

            if &self.client == "podman" {
                build_args.extend(["--format", "docker"]);
            }

            if &self.registry != "localhost" {
                tag_args.extend(vec![image.as_str(), remote_image.as_str()]);
            }

            if self.dryrun {
                println!("{} {}", &self.client, build_args.join(" "));
                if self.registry != "localhost" {
                    println!("{} {}", &self.client, tag_args.join(" "));
                }
            } else {
                let build_output = Command::new(&self.client)
                    .args(build_args)
                    .output()
                    .expect("");
                if self.registry != "localhost" {
                    let tag_output = Command::new(&self.client)
                        .args(tag_args)
                        .output()
                        .expect("");
                }
            }
        }
    }
}

fn main() {
    let extension = "docker";
    let basepath_str = "./integration/";
    let basepath = PathBuf::from(basepath_str);
    let registry = "localhost";
    let client = "podman";
    let dryrun = true;

    let builder = RegistryBuilder::new(registry, client, dryrun, basepath, extension);

    // println!("{:?}", builder.files);
    // println!("{:?}", builder.edges);
    // println!("{:?}", builder.dep_tree);
    println!("{:?}", builder.build_queue);
    builder.build();
}
