Feature: Pull GitHub Issues to TODO.md items

  Scenario: Add open issue to TODO.md when not present
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Existing task
      """
    And there is an open GitHub issue #456 with title "New issue from GitHub"
    When I run `atat pull`
    Then the TODO.md file should contain "- [ ] New issue from GitHub (#456)"
    And cleanup remaining open issues

  Scenario: Check TODO item when corresponding issue is closed
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Task to be completed (#789)
      """
    And GitHub issue #789 is open
    And I update TODO.md to use the actual issue number
    And GitHub issue #789 is closed
    When I run `atat pull`
    Then the TODO.md file should contain "- [x] Task to be completed (#789)"

  Scenario: No changes when TODO.md and GitHub Issues are synchronized
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Open task (#123)
      - [x] Completed task (#456)
      """
    And GitHub issue #123 is open
    And GitHub issue #456 is open
    And I update TODO.md to use the actual issue number
    And GitHub issue #456 is closed
    When I run `atat pull`
    Then the TODO.md file should remain unchanged
    And cleanup remaining open issues

  Scenario: Error when not logged in
    Given the user is not logged in
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] Existing task
      """
    When I run `atat pull`
    Then the error should be "Error: Authentication required"

  Scenario: Error when no repository configured
    Given the user is logged in via GitHub App for tests
    And an empty config file
    And the TODO.md file contains:
      """
      - [ ] Existing task
      """
    When I run `atat pull`
    Then the error should be "Error: No repository configured"

  Scenario: Error when TODO.md file does not exist
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file does not exist
    When I run `atat pull`
    Then the error should be "Error: TODO.md file not found"