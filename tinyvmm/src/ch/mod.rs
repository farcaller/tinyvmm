pub mod bootstrap;
pub mod error;
pub mod runtime;
pub fn get_vm_tap_name(name: &str) -> String {
    use data_encoding::HEXLOWER;
    use ring::digest::{Context, SHA256};

    const PREFIX: &str = "vmi";
    let mut context = Context::new(&SHA256);
    context.update(name.as_bytes());
    let digest = context.finish();
    let hash = HEXLOWER.encode(digest.as_ref());

    // 0123456789012345
    // vmi
    //    VMNAMEVM
    //            DIGES

    format!(
        "{}{}{}",
        PREFIX,
        &name[..std::cmp::min(name.len(), 8)],
        &hash[..5]
    )
}
