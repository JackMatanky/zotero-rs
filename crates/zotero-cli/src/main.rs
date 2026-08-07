//! Command-line interface for the Zotero domain library (scaffold).

#[expect(
    clippy::print_stdout,
    reason = "primary output of a CLI tool is stdout"
)]
fn main() {
    let client = zotero_api::ZoteroClient::default();
    println!("zotero-cli target: {}", client.target_prefix());
}
