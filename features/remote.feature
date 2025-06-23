Feature: List remote repositories

  Scenario: List a single remote repository
    Given the config file content is '{"repositories":["owner/repo1"]}'
    When I run `atat remote`
    Then the output should be "owner/repo1"

  Scenario: List remote repositories when none exist
    Given an empty config file
    When I run `atat remote`
    Then the output should be empty

  Scenario: Add a new repository successfully
    Given the user is logged in via GitHub App for tests
    And an empty config file
    When I run `atat remote add toms74209200/ATAT`
    Then the config file should contain "toms74209200/ATAT"
    And the output should be empty

  Scenario: Attempt to add a repository with an invalid format
    Given an empty config file
    When I run `atat remote add invalid-repo-name`
    Then the error should be "Error: Invalid repository format. Please use <owner>/<repo>."
    And the config file should be empty

  Scenario: Attempt to add an already existing repository
    Given the user is logged in via GitHub App for tests
    And the config file content is '{"repositories":["toms74209200/ATAT"]}'
    When I run `atat remote add toms74209200/ATAT`
    Then the config file should contain "toms74209200/ATAT"
    And the output should be empty

  Scenario: Attempt to add a non-existent repository
    Given the user is logged in via GitHub App for tests
    And an empty config file
    When I run `atat remote add non-existent-owner/non-existent-repo`
    Then the error should be "Error: Repository non-existent-owner/non-existent-repo not found or not accessible."
    And the config file should be empty

  Scenario: Remove a repository successfully
    Given the config file content is '{"repositories":["owner/repo1","owner/repo2"]}'
    When I run `atat remote remove owner/repo1`
    Then the config file should contain "owner/repo2"
    And the output should be empty

  Scenario: Remove the last repository
    Given the config file content is '{"repositories":["owner/repo1"]}'
    When I run `atat remote remove owner/repo1`
    Then the config file should be empty
    And the output should be empty

  Scenario: Remove repository from empty configuration
    Given an empty config file
    When I run `atat remote remove owner/repo`
    Then the output should be empty
    And the config file should be empty

  Scenario: Remove non-existent repository
    Given the config file content is '{"repositories":["owner/repo1"]}'
    When I run `atat remote remove owner/repo2`
    Then the output should be empty
    And the config file should contain "owner/repo1"
