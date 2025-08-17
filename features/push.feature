Feature: Push TODO.md items to GitHub Issues

  Scenario: Create new issue for unchecked TODO item
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] New task to implement
      """
    When I run `atat push`
    Then a new GitHub issue should be created with title "New task to implement"
    And the TODO.md file should be updated with the issue number
    And cleanup remaining open issues

  Scenario: Close issue for checked TODO item
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [x] Completed task (#123)
      """
    And GitHub issue #123 is open
    And I update TODO.md to use the actual issue number
    When I run `atat push`
    Then the created issue should be closed

  Scenario: Error when not logged in
    Given the user is not logged in
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file contains:
      """
      - [ ] New task
      """
    When I run `atat push`
    Then the error should be "Error: Authentication required"

  Scenario: Error when no repository configured
    Given the user is logged in via GitHub App for tests
    And an empty config file
    And the TODO.md file contains:
      """
      - [ ] New task
      """
    When I run `atat push`
    Then the error should be "Error: No repository configured"

  Scenario: Error when TODO.md file does not exist
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/atat-test"]}'
    And the TODO.md file does not exist
    When I run `atat push`
    Then the error should be "Error: TODO.md file not found"
