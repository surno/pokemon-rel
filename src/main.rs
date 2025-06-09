use pokebot_rust::NetworkManager;

fn main() {
    let manager = NetworkManager::new();
    manager.start();
}
