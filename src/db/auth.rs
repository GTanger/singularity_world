//! 密碼（bcrypt）—— 直走 pg pool，不經 store lock。

use super::ErrNoStore;

const BCRYPT_COST: u32 = 10;

/// 建立密碼雜湊。
pub fn create_auth(entity_id: &str, password: &str) -> anyhow::Result<()> {
    let pool = crate::pg::pool().ok_or(ErrNoStore)?;
    let hash = bcrypt::hash(password, BCRYPT_COST)?;
    crate::pg::auth::set(&pool, entity_id, &hash)
}

/// 是否已有密碼。
pub fn has_password_for_entity(entity_id: &str) -> bool {
    match crate::pg::pool() {
        Some(pool) => !crate::pg::auth::get(&pool, entity_id).is_empty(),
        None => false,
    }
}

/// 驗證密碼。
pub fn verify_password(entity_id: &str, password: &str) -> anyhow::Result<bool> {
    let pool = crate::pg::pool().ok_or(ErrNoStore)?;
    let hash = crate::pg::auth::get(&pool, entity_id);
    if hash.is_empty() {
        return Ok(false);
    }
    Ok(bcrypt::verify(password, &hash).unwrap_or(false))
}
