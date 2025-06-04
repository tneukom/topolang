For some reason when using tracy-client the stacktrace on Windows 11 is missing.
The issue is mentioned here but is closed:
https://github.com/nagisa/rust_tracy_client/pull/122

Disabling tracy by removing the "enable" feature flag solves the issue. 