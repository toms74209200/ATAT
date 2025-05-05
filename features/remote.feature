Feature: List remote repositories

  Scenario: List a single remote repository
    Given the config file content is '{"repositories":["owner/repo1"]}'
    When I run `atat remote`
    Then the output should be "owner/repo1"

  Scenario: List remote repositories when none exist
    Given an empty config file
    When I run `atat remote`
    Then the output should be empty
