// This example shows the main modules available in trippy
fn main() {
    println!("Available modules in trippy:");
    println!("- trippy::core");
    println!("- trippy::dns");
    println!("- trippy::packet");
    println!("- trippy::privilege");
    println!("- trippy::tui");
    
    // Print version if available
    println!("\nTrippy version: {}", env!("CARGO_PKG_VERSION"));
}
