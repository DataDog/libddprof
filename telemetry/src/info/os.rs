// TODO: this function will call API's in the future to get to real host API
pub async fn real_hostname() -> anyhow::Result<String> {
    Ok(sys_info::hostname()?)
}

pub const fn os_name() -> &'static str {
    std::env::consts::OS
}

pub fn os_version() -> anyhow::Result<String> {
    sys_info::os_release().map_err(|e| e.into())
}
