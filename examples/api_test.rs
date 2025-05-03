// This example shows how to list the modules available in trippy
fn main() {
    println!("Trippy modules:");
    // List known modules in trippy
    let modules = vec![
        "trippy::core",
        "trippy::dns",
        "trippy::packet",
        "trippy::privilege",
        "trippy::tui"
    ];
    
    for module in modules {
        println!("- {}", module);
    }
    
    // Print some information about the trippy version
    println!("\nUsing trippy version: {}", env!("CARGO_PKG_VERSION"));
}
