use crate::todo::TodoItem;
use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser};

pub fn parse_todo_markdown(content: &str) -> Result<Vec<TodoItem>> {
    let (items, _, _) = Parser::new_ext(
        content,
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    )
    .fold(
        (Vec::new(), None::<bool>, String::new()),
        |(mut items, pending_checked, mut text_buffer), event| match event {
            Event::TaskListMarker(checked) => (items, Some(checked), String::new()),
            Event::Text(text) if pending_checked.is_some() => {
                text_buffer.push_str(&text);
                (items, pending_checked, text_buffer)
            }
            Event::Code(text) if pending_checked.is_some() => {
                text_buffer.push_str(&text);
                (items, pending_checked, text_buffer)
            }
            Event::Start(pulldown_cmark::Tag::List(_))
                if pending_checked.is_some() && !text_buffer.is_empty() =>
            {
                let is_checked = pending_checked.unwrap();
                let text_str = text_buffer.trim();

                let (clean_text, issue_number) = text_str
                    .rfind(" (#")
                    .and_then(|pos| {
                        text_str[pos..].find(')').and_then(|end_pos| {
                            let issue_part = &text_str[pos + 3..pos + end_pos];
                            issue_part
                                .parse::<u64>()
                                .ok()
                                .map(|num| (text_str[..pos].trim().to_string(), Some(num)))
                        })
                    })
                    .unwrap_or_else(|| (text_str.to_string(), None));

                items.push(TodoItem {
                    text: clean_text,
                    is_checked,
                    issue_number,
                });

                (items, None, String::new())
            }
            Event::End(pulldown_cmark::TagEnd::Item) if !text_buffer.is_empty() => {
                if let Some(is_checked) = pending_checked {
                    let text_str = text_buffer.trim();

                    let (clean_text, issue_number) = text_str
                        .rfind(" (#")
                        .and_then(|pos| {
                            text_str[pos..].find(')').and_then(|end_pos| {
                                let issue_part = &text_str[pos + 3..pos + end_pos];
                                issue_part
                                    .parse::<u64>()
                                    .ok()
                                    .map(|num| (text_str[..pos].trim().to_string(), Some(num)))
                            })
                        })
                        .unwrap_or_else(|| (text_str.to_string(), None));

                    items.push(TodoItem {
                        text: clean_text,
                        is_checked,
                        issue_number,
                    });
                }

                (items, None, String::new())
            }
            _ => (items, pending_checked, text_buffer),
        },
    );

    Ok(items)
}

pub fn serialize_todo_markdown(items: &[TodoItem]) -> String {
    items
        .iter()
        .map(|item| {
            let checkbox = if item.is_checked { "[x]" } else { "[ ]" };
            let text = if let Some(issue_number) = item.issue_number {
                format!("{} (#{issue_number})", item.text)
            } else {
                item.text.clone()
            };
            format!("- {checkbox} {text}\n")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_checklist_exact_text() {
        let content = r#"- [ ] Task 1
- [x] Task 2
- [X] Task 3"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].text, "Task 1");
        assert!(!items[0].is_checked);
        assert_eq!(items[1].text, "Task 2");
        assert!(items[1].is_checked);
        assert_eq!(items[2].text, "Task 3");
        assert!(items[2].is_checked);
    }

    #[test]
    fn test_text_formatting_exact_match() {
        let content = r#"- [ ] **bold** text
- [x] *italic* text
- [ ] `code` text
- [x] [link](url) text
- [ ] ~~strikethrough~~ text"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 5);
        assert_eq!(items[0].text, "bold text");
        assert_eq!(items[1].text, "italic text");
        assert_eq!(items[2].text, "code text");
        assert_eq!(items[3].text, "link text");
        assert_eq!(items[4].text, "strikethrough text");
    }

    #[test]
    fn test_issue_numbers_exact_text() {
        let content = r#"- [ ] Task with issue (#123)
- [x] Another task (#456)
- [ ] Task without issue"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].text, "Task with issue");
        assert_eq!(items[0].issue_number, Some(123));
        assert_eq!(items[1].text, "Another task");
        assert_eq!(items[1].issue_number, Some(456));
        assert_eq!(items[2].text, "Task without issue");
        assert_eq!(items[2].issue_number, None);
    }

    #[test]
    fn test_nested_checklist_flat_structure() {
        let content = r#"- [ ] Main task
  - [ ] Sub task 1
  - [x] Sub task 2
    - [ ] Sub sub task
- [x] Another main task"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 5);
        assert_eq!(items[0].text, "Main task");
        assert!(!items[0].is_checked);
        assert_eq!(items[1].text, "Sub task 1");
        assert!(!items[1].is_checked);
        assert_eq!(items[2].text, "Sub task 2");
        assert!(items[2].is_checked);
        assert_eq!(items[3].text, "Sub sub task");
        assert!(!items[3].is_checked);
        assert_eq!(items[4].text, "Another main task");
        assert!(items[4].is_checked);
    }

    #[test]
    fn test_sections_with_checklist() {
        let content = r#"# Section 1

- [x] Completed task
- [ ] Pending task

## Subsection

- [x] Another completed
- [ ] Another pending"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].text, "Completed task");
        assert!(items[0].is_checked);
        assert_eq!(items[1].text, "Pending task");
        assert!(!items[1].is_checked);
        assert_eq!(items[2].text, "Another completed");
        assert!(items[2].is_checked);
        assert_eq!(items[3].text, "Another pending");
        assert!(!items[3].is_checked);
    }

    #[test]
    fn test_mixed_content_ignore_non_checklist() {
        let content = r#"# Title

Regular text.

- Regular bullet
- Another bullet

- [ ] Checklist item 1
- [x] Checklist item 2

```
code block
```

- [ ] Checklist item 3"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].text, "Checklist item 1");
        assert!(!items[0].is_checked);
        assert_eq!(items[1].text, "Checklist item 2");
        assert!(items[1].is_checked);
        assert_eq!(items[2].text, "Checklist item 3");
        assert!(!items[2].is_checked);
    }

    #[test]
    fn test_empty_and_whitespace() {
        let content = "";
        let items = parse_todo_markdown(content).unwrap();
        assert_eq!(items.len(), 0);

        let content_whitespace = r#"   


  "#;
        let items = parse_todo_markdown(content_whitespace).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_invalid_issue_format() {
        let content = r#"- [ ] Task (#invalid)
- [x] Task (#)
- [ ] Task (# 123)
- [x] Valid task (#456)"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].text, "Task (#invalid)");
        assert_eq!(items[0].issue_number, None);
        assert_eq!(items[1].text, "Task (#)");
        assert_eq!(items[1].issue_number, None);
        assert_eq!(items[2].text, "Task (# 123)");
        assert_eq!(items[2].issue_number, None);
        assert_eq!(items[3].text, "Valid task");
        assert_eq!(items[3].issue_number, Some(456));
    }

    #[test]
    fn test_special_characters_in_text() {
        let content = r#"- [ ] Task with emoji 🚀
- [x] Task with symbols !@#$%
- [ ] Task with Japanese 日本語
- [x] Task with numbers 123"#;

        let items = parse_todo_markdown(content).unwrap();

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].text, "Task with emoji 🚀");
        assert_eq!(items[1].text, "Task with symbols !@#$%");
        assert_eq!(items[2].text, "Task with Japanese 日本語");
        assert_eq!(items[3].text, "Task with numbers 123");
    }

    #[test]
    fn test_serialize_todo_markdown() {
        let items = vec![
            TodoItem {
                text: "Unchecked task".to_string(),
                is_checked: false,
                issue_number: None,
            },
            TodoItem {
                text: "Checked task".to_string(),
                is_checked: true,
                issue_number: None,
            },
            TodoItem {
                text: "Task with issue".to_string(),
                is_checked: false,
                issue_number: Some(123),
            },
            TodoItem {
                text: "Checked task with issue".to_string(),
                is_checked: true,
                issue_number: Some(456),
            },
        ];

        let expected = "- [ ] Unchecked task\n- [x] Checked task\n- [ ] Task with issue (#123)\n- [x] Checked task with issue (#456)\n";
        let actual = serialize_todo_markdown(&items);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_serialize_empty_list() {
        let items = vec![];
        let actual = serialize_todo_markdown(&items);
        assert_eq!(actual, "");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let original_content = "- [ ] Task 1\n- [x] Task 2 (#123)\n- [ ] Task 3\n";
        let parsed_items = parse_todo_markdown(original_content).unwrap();
        let serialized = serialize_todo_markdown(&parsed_items);

        assert_eq!(serialized, original_content);
    }
}
