Feature: Clean completed items from TODO.md

  Scenario: Remove checked item with closed issue
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Completed task (#100)
      """
    And GitHub issue #100 with title "Completed task"
    And I update TODO.md to use the actual issue number
    And GitHub issue #100 is closed
    When I run `atat clean`
    Then the TODO.md file should not contain "Completed task"
    And cleanup remaining open issues

  Scenario: Remove multiple checked items with closed issues
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] First task (#100)
      - [x] Second task (#101)
      """
    And GitHub issue #100 with title "First task"
    And GitHub issue #101 with title "Second task"
    And I update TODO.md to use the actual issue number
    And GitHub issue #100 is closed
    And GitHub issue #101 is closed
    When I run `atat clean`
    Then the TODO.md file should not contain "First task"
    And the TODO.md file should not contain "Second task"
    And cleanup remaining open issues

  Scenario: Skip checked item with open issue
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Checked but open (#100)
      - [x] Checked and closed (#101)
      """
    And GitHub issue #100 with title "Checked but open"
    And GitHub issue #101 with title "Checked and closed"
    And I update TODO.md to use the actual issue number
    And GitHub issue #101 is closed
    When I run `atat clean`
    Then the TODO.md file should contain "Checked but open"
    And the TODO.md file should not contain "Checked and closed"
    And cleanup remaining open issues

  Scenario: Skip checked item without issue number
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Task without issue
      - [x] Task with issue (#100)
      """
    And GitHub issue #100 with title "Task with issue"
    And I update TODO.md to use the actual issue number
    And GitHub issue #100 is closed
    When I run `atat clean`
    Then the TODO.md file should contain "Task without issue"
    And the TODO.md file should not contain "Task with issue"
    And cleanup remaining open issues

  Scenario: Skip unchecked item with closed issue
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Unchecked task (#100)
      """
    And GitHub issue #100 with title "Unchecked task"
    And I update TODO.md to use the actual issue number
    And GitHub issue #100 is closed
    When I run `atat clean`
    Then the TODO.md file should contain "Unchecked task"
    And cleanup remaining open issues

  Scenario: No changes when no items to remove
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Open task (#100)
      """
    And GitHub issue #100 with title "Open task"
    And I update TODO.md to use the actual issue number
    When I run `atat clean`
    Then the TODO.md file should remain unchanged
    And cleanup remaining open issues

  Scenario: Dry run mode
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Completed task (#100)
      """
    And GitHub issue #100 with title "Completed task"
    And I update TODO.md to use the actual issue number
    And GitHub issue #100 is closed
    When I run `atat clean --dry-run`
    Then the TODO.md file should remain unchanged
    And cleanup remaining open issues

  Scenario: Error when not logged in
    Given the user is not logged in
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Completed task (#100)
      """
    When I run `atat clean`
    Then the error should be "Error: Authentication required"

  Scenario: Error when no repository configured
    Given the user is logged in via GitHub App for tests
    And an empty config file
    And the TODO.md file contains:
      """
      - [x] Completed task (#100)
      """
    When I run `atat clean`
    Then the error should be "Error: No repository configured"

  Scenario: Error when TODO.md file does not exist
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file does not exist
    When I run `atat clean`
    Then the error should be "Error: TODO.md file not found"
