pub const REAL_PROTOCOL_ENV: &str = "SJTU_REAL_PROTOCOL_TEST";

pub fn real_protocol_enabled(value: Option<&str>) -> bool {
    value == Some("1")
}

pub fn ensure_real_protocol_enabled() -> Result<(), &'static str> {
    let value = std::env::var(REAL_PROTOCOL_ENV).ok();
    if real_protocol_enabled(value.as_deref()) {
        return Ok(());
    }
    Err("真实协议验证默认关闭；请显式设置 SJTU_REAL_PROTOCOL_TEST=1。")
}
