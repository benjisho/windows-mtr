fn main() { println!("Available in trippy:"); for n in trippy::module_path().split("::") { println!("- {}", n); } }
