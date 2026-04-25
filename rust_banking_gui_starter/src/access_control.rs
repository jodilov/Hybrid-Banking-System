use crate::models::User;

pub fn can_view_account(current_user: &User, owner_user_id: i32) -> bool {
    match current_user.role.as_str() {
        "customer" => current_user.user_id == owner_user_id,
        "teller" | "manager" | "auditor" => true,
        _ => false,
    }
}

pub fn can_initiate_transfer(current_user: &User, owner_user_id: i32) -> bool {
    match current_user.role.as_str() {
        "customer" => current_user.user_id == owner_user_id,
        "teller" | "manager" => true,
        _ => false,
    }
}

pub fn can_approve_transfer(current_user: &User) -> bool {
    current_user.role == "manager"
}

pub fn can_view_audit_logs(current_user: &User) -> bool {
    current_user.role == "auditor" || current_user.role == "manager"
}
