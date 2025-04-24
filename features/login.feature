Feature: User Authentication with GitHub Account

  Scenario: Initiate Device Flow
    Given the user is not logged in
    When the user executes the `atat login` command
    Then the authentication URL "https://github.com/login/device" should be displayed on standard output
    And a user code consisting of 8 alphanumeric characters and a hyphen should be displayed on standard output
    And a message prompting for browser authentication should be displayed on standard output

  Scenario: Complete Device Flow Authentication
    Given the user has executed `atat login` and the URL and user code are displayed
    When the test runner completes the GitHub device authentication flow using the displayed information
    Then a login success message should be displayed on standard output
