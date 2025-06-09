use pokebot_rust::NetworkManager;

#[tokio::main]
async fn main() {
    let (mut manager, handle) = NetworkManager::new(3344);
    let result = manager.start().await;
    if result.is_err() {
        println!("Error starting manager: {:?}", result.err());
        return;
    }
    println!("Manager started on port 3344");
}
