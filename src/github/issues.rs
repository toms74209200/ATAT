#[derive(Debug, Clone, PartialEq)]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    pub state: IssueState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueState {
    Open,
    Closed,
}
