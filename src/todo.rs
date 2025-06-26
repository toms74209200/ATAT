#[derive(Debug, Clone, PartialEq)]
pub struct TodoItem {
    pub text: String,
    pub is_checked: bool,
    pub issue_number: Option<u64>,
}
